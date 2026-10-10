//! 감시 루프(스펙 5절). 앱을 띄워 [Config::tick]마다 보고, 충돌·멈춤이면 기다렸다 다시 띄운다.
//! 운영자가 닫았으면(clean_exit의 PID가 이 앱) 다시 띄우지 않고 감시도 끝낸다.
use crate::backoff::Backoff;
use crate::heartbeat::{read_heartbeat, read_pid, CLEAN_EXIT, HEARTBEAT, SUPERVISOR_LOCK};
use crate::logfile::log;
use crate::process::{adopt_running_app, Target};
use crate::watch::{Verdict, Watch};
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// 오디오 멈춤으로는 연속 이 횟수까지만 다시 띄운다(스펙 5.3).
pub const AUDIO_STALL_LIMIT: u32 = 3;
/// 앱이 중복 실행이라 끝날 때의 종료 코드(다른 앱이 이미 떠 있다).
pub const DUPLICATE_EXIT_CODE: i32 = 3;
/// 중복 종료가 이만큼 이어지면(넘겨받을 앱을 못 찾음, 권한 등) 바로 다시 보지 않고 재실행 대기를 둔다.
const DUPLICATE_RETRY_LIMIT: u32 = 3;

#[cfg(windows)]
const DEFAULT_APP: &str = "atmos_mixer_pro.exe";
#[cfg(not(windows))]
const DEFAULT_APP: &str = "atmos_mixer_pro";

/// 감시 설정. 기본값은 스펙 5.1이다. 시간 인자는 테스트와 macOS 확인에서 짧게 준다.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// 띄울 앱(기본: 감시 프로그램과 같은 폴더의 앱).
    pub app: PathBuf,
    /// 신호 파일과 감시 기록을 두는 폴더(기본: 앱 로그 폴더).
    pub dir: PathBuf,
    /// 로그인 자동 실행으로 시작했다(첫 실행에 --logon을 넘긴다).
    pub logon: bool,
    /// UI 하트비트가 이만큼 안 바뀌면 멈춤.
    pub hang: Duration,
    /// 새로 띄운 앱의 첫 하트비트 기한.
    pub first_heartbeat: Duration,
    /// 엔진이 켜져 있는데 오디오 콜백 시각이 이만큼 안 바뀌면 오디오 멈춤.
    pub audio_stall: Duration,
    /// 확인 간격.
    pub tick: Duration,
    pub backoff_min: Duration,
    pub backoff_max: Duration,
    /// 앱이 이만큼 잘 돌았으면 대기 시간을 처음 값으로 돌린다.
    pub stable: Duration,
}

/// 앱 로그 폴더. 앱의 `core::log_file::log_dir()`(rust/src/core/log_file.rs)와 같아야 한다 —
/// 환경 변수 ATMOS_LOG_DIR이 있으면 앱과 감시 모두 그 폴더를 쓴다.
pub fn default_dir() -> PathBuf {
    match std::env::var_os("ATMOS_LOG_DIR") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => std::env::temp_dir().join("atmos_mixer_pro_logs"),
    }
}

impl Config {
    /// 인자를 읽는다. [exe_dir]는 감시 프로그램이 있는 폴더다.
    pub fn parse(args: &[String], exe_dir: &Path) -> Result<Config, String> {
        let mut cfg = Config {
            app: exe_dir.join(DEFAULT_APP),
            dir: default_dir(),
            logon: false,
            hang: Duration::from_secs(30),
            first_heartbeat: Duration::from_secs(90),
            audio_stall: Duration::from_secs(120),
            tick: Duration::from_secs(1),
            backoff_min: Duration::from_secs(2),
            backoff_max: Duration::from_secs(60),
            stable: Duration::from_secs(300),
        };
        let mut it = args.iter();
        while let Some(arg) = it.next() {
            match arg.as_str() {
                "--logon" => cfg.logon = true,
                "--app" => cfg.app = PathBuf::from(value(&mut it, arg)?),
                "--dir" => cfg.dir = PathBuf::from(value(&mut it, arg)?),
                "--hang-secs" => cfg.hang = secs(arg, value(&mut it, arg)?)?,
                "--first-heartbeat-secs" => cfg.first_heartbeat = secs(arg, value(&mut it, arg)?)?,
                "--audio-stall-secs" => cfg.audio_stall = secs(arg, value(&mut it, arg)?)?,
                "--tick-ms" => cfg.tick = millis(arg, value(&mut it, arg)?)?,
                "--backoff-min-ms" => cfg.backoff_min = millis(arg, value(&mut it, arg)?)?,
                other => return Err(format!("모르는 인자: {other}")),
            }
        }
        Ok(cfg)
    }
}

fn value<'a>(it: &mut std::slice::Iter<'a, String>, name: &str) -> Result<&'a str, String> {
    it.next().map(String::as_str).ok_or_else(|| format!("{name} 뒤에 값이 없다"))
}

fn secs(name: &str, text: &str) -> Result<Duration, String> {
    match text.parse::<f64>() {
        Ok(v) if v.is_finite() && v > 0.0 => Ok(Duration::from_secs_f64(v)),
        _ => Err(format!("{name} 값이 잘못됐다: {text}")),
    }
}

fn millis(name: &str, text: &str) -> Result<Duration, String> {
    match text.parse::<u64>() {
        Ok(v) if v > 0 => Ok(Duration::from_millis(v)),
        _ => Err(format!("{name} 값이 잘못됐다: {text}")),
    }
}

/// 감시 대상 하나를 지켜본 결과.
enum Ending {
    /// 프로세스가 끝났다(종료 코드를 모르면 None).
    Exited(Option<i32>),
    /// 멈춤으로 판단했다(아직 살아 있다).
    Hung(Verdict),
}

struct Supervisor<'a> {
    cfg: &'a Config,
    backoff: Backoff,
    /// 다음 실행에 --auto-relaunched를 붙인다(충돌·멈춤 뒤).
    relaunch: bool,
    /// 아직 앱을 한 번도 띄우지 못했다(--logon은 첫 실행에만 넘긴다).
    first_launch: bool,
    /// 오디오 멈춤으로 연속 다시 띄운 횟수. 콜백이 다시 돌면 0으로 돌아간다.
    audio_relaunches: u32,
    /// 한도에 걸린 뒤 "더는 다시 띄우지 않는다"를 한 번만 남긴다.
    audio_gave_up_logged: bool,
    /// 마지막으로 본 무음 경고 누적 횟수.
    silent_warnings: u32,
    /// 중복 실행(종료 코드 3)으로 연달아 끝난 횟수.
    duplicates: u32,
}

/// 감시를 돌린다. 운영자가 앱을 닫았거나 감시가 이미 돌고 있으면 돌아온다.
pub fn run(cfg: &Config) {
    let _ = std::fs::create_dir_all(&cfg.dir);
    let Some(_lock) = acquire_lock(&cfg.dir.join(SUPERVISOR_LOCK)) else {
        hand_to_running_supervisor(cfg);
        return;
    };
    log(
        &cfg.dir,
        &format!(
            "감시 시작(앱 {}, 멈춤 {:?}, 첫 하트비트 {:?}, 오디오 멈춤 {:?}{})",
            cfg.app.display(),
            cfg.hang,
            cfg.first_heartbeat,
            cfg.audio_stall,
            if cfg.logon { ", 로그인 자동 실행" } else { "" }
        ),
    );
    Supervisor {
        cfg,
        backoff: Backoff::new(cfg.backoff_min, cfg.backoff_max, cfg.stable),
        relaunch: false,
        first_launch: true,
        audio_relaunches: 0,
        audio_gave_up_logged: false,
        silent_warnings: 0,
        duplicates: 0,
    }
    .run_loop();
}

/// 감시가 이미 돈다(바로가기를 또 눌렀다). 앱을 인자 없이 한 번 띄우면 앱의 시작 관문이 기존 창을 앞으로
/// 가져오고 끝난다. 앱이 내려가 있던 사이(재실행 대기 중)라면 앱이 바로 뜨고, 돌고 있는 감시가 넘겨받는다.
/// 띄운 앱은 기다리지 않는다 — 감시가 곧 끝나므로 init이 거둔다.
#[allow(clippy::zombie_processes)]
fn hand_to_running_supervisor(cfg: &Config) {
    match Target::spawn(&cfg.app, &[]) {
        Ok(target) => log(
            &cfg.dir,
            &format!("감시가 이미 돌고 있다 — 앱을 한 번 띄워(pid {}) 기존 창을 앞으로 가져오고 끝낸다", target.pid()),
        ),
        Err(e) => log(&cfg.dir, &format!("감시가 이미 돌고 있다 — 앱을 띄우지 못했다: {e}")),
    }
}

/// [path]를 열어 잠근다. 다른 프로세스가 쥐고 있으면 None.
fn acquire_lock(path: &Path) -> Option<File> {
    let file = OpenOptions::new().create(true).truncate(false).read(true).write(true).open(path).ok()?;
    file.try_lock().ok()?;
    Some(file)
}

impl Supervisor<'_> {
    fn run_loop(&mut self) {
        loop {
            let Some((mut target, adopted)) = self.next_target() else {
                // 앱을 띄우지 못했다(파일 없음 등). 기다렸다 다시 해 본다.
                self.wait_before_relaunch(Duration::ZERO);
                continue;
            };
            let started = Instant::now();
            match self.watch(&mut target, adopted) {
                Ending::Exited(code) if self.closed_by_operator(&target) => {
                    self.log(&format!("운영자가 앱을 닫았다(pid {}, 종료 코드 {code:?}) — 감시를 끝낸다", target.pid()));
                    return;
                }
                Ending::Exited(Some(DUPLICATE_EXIT_CODE)) => {
                    // 다른 앱이 이미 떠 있다. 바로 대상을 다시 정한다(넘겨받기).
                    self.duplicates += 1;
                    if self.duplicates < DUPLICATE_RETRY_LIMIT {
                        self.log("앱이 중복 실행으로 끝났다(종료 코드 3) — 떠 있는 앱을 넘겨받는다");
                        continue;
                    }
                    self.log("앱이 중복 실행으로 끝나는데 넘겨받을 앱이 보이지 않는다 — 기다렸다 다시 본다");
                    self.wait_before_relaunch(Duration::ZERO);
                    continue;
                }
                Ending::Exited(code) => {
                    self.log(&format!(
                        "앱이 닫는다는 표시 없이 끝났다(충돌, pid {}, 종료 코드 {code:?})",
                        target.pid()
                    ));
                }
                Ending::Hung(verdict) => {
                    let closing = self.closed_by_operator(&target);
                    target.kill();
                    if closing {
                        self.log(&format!(
                            "운영자가 닫는 중 멈춘 앱을 강제 종료했다(pid {}) — 감시를 끝낸다",
                            target.pid()
                        ));
                        return;
                    }
                    if verdict == Verdict::AudioStall {
                        self.audio_relaunches += 1;
                    }
                    self.log(&format!("{} — 앱을 강제 종료했다(pid {})", self.describe(verdict), target.pid()));
                }
            }
            self.duplicates = 0;
            self.relaunch = true;
            self.wait_before_relaunch(started.elapsed());
        }
    }

    /// 다음 감시 대상: (대상, 넘겨받았나). 떠 있는 앱이 있으면 넘겨받고, 없으면 띄운다.
    fn next_target(&mut self) -> Option<(Target, bool)> {
        if let Some(target) = adopt_running_app(&self.cfg.dir) {
            self.log(&format!("떠 있는 앱을 넘겨받았다(pid {})", target.pid()));
            self.duplicates = 0;
            return Some((target, true));
        }
        self.launch().map(|target| (target, false))
    }

    fn launch(&mut self) -> Option<Target> {
        // 지난 앱의 신호를 새 앱의 것으로 읽지 않게 지운다.
        let _ = std::fs::remove_file(self.cfg.dir.join(HEARTBEAT));
        let _ = std::fs::remove_file(self.cfg.dir.join(CLEAN_EXIT));
        let mut args = vec!["--supervised".to_string()];
        if self.relaunch {
            args.push("--auto-relaunched".into());
        }
        if self.first_launch && self.cfg.logon {
            args.push("--logon".into());
        }
        match Target::spawn(&self.cfg.app, &args) {
            Ok(target) => {
                self.first_launch = false;
                self.log(&format!("앱을 띄웠다(pid {}, 인자 {})", target.pid(), args.join(" ")));
                Some(target)
            }
            Err(e) => {
                self.log(&format!("앱을 띄우지 못했다({}): {e}", self.cfg.app.display()));
                None
            }
        }
    }

    /// 앱이 끝나거나 멈출 때까지 본다.
    fn watch(&mut self, target: &mut Target, adopted: bool) -> Ending {
        // 넘겨받은 앱은 하트비트가 이미 있어야 하므로 첫 하트비트 기한을 멈춤 판단과 같게 둔다.
        let first = if adopted { self.cfg.hang } else { self.cfg.first_heartbeat };
        let mut watch = Watch::new(Instant::now(), first, self.cfg.hang, self.cfg.audio_stall);
        loop {
            std::thread::sleep(self.cfg.tick);
            if let Some(code) = target.try_exit() {
                return Ending::Exited(code);
            }
            let heartbeat = read_heartbeat(&self.cfg.dir).filter(|h| h.pid == target.pid());
            if let Some(h) = &heartbeat {
                self.note_silent_warnings(h.silent_warnings);
            }
            let seen = watch.observe(Instant::now(), heartbeat.as_ref());
            if seen.gap {
                self.log("확인 간격이 크게 벌어졌다(절전에서 깨어남 등) — 멈춤 판단 시간을 다시 잰다");
            }
            if seen.audio_progressed && self.audio_relaunches > 0 {
                self.log("오디오 콜백이 다시 돈다 — 오디오 멈춤 재실행 횟수를 초기화한다");
                self.audio_relaunches = 0;
                self.audio_gave_up_logged = false;
            }
            match seen.verdict {
                Verdict::Ok => {}
                Verdict::AudioStall if self.audio_relaunches >= AUDIO_STALL_LIMIT => {
                    if !self.audio_gave_up_logged {
                        self.log(&format!(
                            "오디오 멈춤이 이어지지만 연속 {AUDIO_STALL_LIMIT}번 다시 띄워도 그대로라 \
                             더는 오디오 멈춤으로 다시 띄우지 않는다(장치를 확인하세요)"
                        ));
                        self.audio_gave_up_logged = true;
                    }
                }
                verdict => return Ending::Hung(verdict),
            }
        }
    }

    fn closed_by_operator(&self, target: &Target) -> bool {
        read_pid(&self.cfg.dir.join(CLEAN_EXIT)) == Some(target.pid())
    }

    fn note_silent_warnings(&mut self, count: u32) {
        if count > self.silent_warnings {
            self.log(&format!("앱이 재생 중 무음 경고를 남겼다(누적 {count}회 — 앱 로그 참고)"));
        }
        self.silent_warnings = count;
    }

    fn describe(&self, verdict: Verdict) -> String {
        match verdict {
            Verdict::StartupHang => "시작 중 멈춤: 첫 하트비트가 기한 안에 오지 않았다".into(),
            Verdict::UiHang => {
                format!("UI 멈춤: 하트비트가 {:.0}초 넘게 바뀌지 않았다", self.cfg.hang.as_secs_f64())
            }
            Verdict::AudioStall => format!(
                "오디오 멈춤: 엔진이 켜져 있는데 오디오 콜백이 {:.0}초 넘게 돌지 않았다(연속 {}번째)",
                self.cfg.audio_stall.as_secs_f64(),
                self.audio_relaunches
            ),
            Verdict::Ok => "정상".into(),
        }
    }

    fn wait_before_relaunch(&mut self, uptime: Duration) {
        let delay = self.backoff.next_delay(uptime);
        self.log(&format!("{:.1}초 뒤 다시 띄운다", delay.as_secs_f64()));
        std::thread::sleep(delay);
    }

    fn log(&self, msg: &str) {
        log(&self.cfg.dir, msg);
    }
}
