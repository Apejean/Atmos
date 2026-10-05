//! 감시 프로그램을 실제 프로세스로 띄워 확인한다. 앱 대신 가짜 앱(src/bin/fake_app.rs)을 띄운다.
//! 판단 시간은 짧게 준다(멈춤·첫 하트비트·오디오 멈춤 2초, 확인 0.1초, 첫 대기 0.2초).
//! 테스트마다 폴더가 따로라 함께 돌아도 된다.
use atmos_supervisor::heartbeat::{read_pid, APP_LOCK, APP_PID};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const SUPERVISOR: &str = env!("CARGO_BIN_EXE_atmos_supervisor");
const FAKE_APP: &str = env!("CARGO_BIN_EXE_fake_app");

fn fresh_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("atmos_sup_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// 띄운 감시 프로그램. 테스트가 실패해도 남지 않게 drop에서 끝낸다.
struct Supervised(Child);

impl Supervised {
    fn wait_exit(&mut self, secs: u64, what: &str) -> ExitStatus {
        let end = Instant::now() + Duration::from_secs(secs);
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                return status;
            }
            assert!(Instant::now() < end, "{what}");
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn running(&mut self) -> bool {
        self.0.try_wait().unwrap().is_none()
    }
}

impl Drop for Supervised {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// 감시 프로그램을 띄운다. [env]는 가짜 앱 동작(FAKE_FIRST 등)이고 감시가 띄우는 앱이 물려받는다.
fn supervisor(dir: &Path, extra: &[&str], env: &[(&str, &str)]) -> Supervised {
    supervisor_with_app(Path::new(FAKE_APP), None, dir, extra, env)
}

/// [app] 경로로 앱을 띄우는 감시 프로그램. [cwd]가 있으면 그 폴더에서 감시를 띄운다.
fn supervisor_with_app(
    app: &Path,
    cwd: Option<&Path>,
    dir: &Path,
    extra: &[&str],
    env: &[(&str, &str)],
) -> Supervised {
    let mut command = Command::new(SUPERVISOR);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command
        .arg("--app")
        .arg(app)
        .arg("--dir")
        .arg(dir)
        .args(["--hang-secs", "2", "--first-heartbeat-secs", "2", "--audio-stall-secs", "2"])
        .args(["--tick-ms", "100", "--backoff-min-ms", "200"])
        .args(extra)
        .env("FAKE_DIR", dir)
        .env_remove("FAKE_MODE")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for (key, value) in env {
        command.env(key, value);
    }
    Supervised(command.spawn().unwrap())
}

/// 가짜 앱 실행 기록(줄마다 "PID 인자…")
fn launches(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("launches")).unwrap_or_default().lines().map(str::to_owned).collect()
}

/// 실행 기록의 인자 부분
fn launch_args(dir: &Path) -> Vec<String> {
    launches(dir)
        .iter()
        .map(|line| line.split_once(' ').map(|(_, args)| args.to_owned()).unwrap_or_default())
        .collect()
}

fn log_text(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("supervisor.log")).unwrap_or_default()
}

fn wait_until(what: &str, secs: u64, cond: impl Fn() -> bool) {
    let end = Instant::now() + Duration::from_secs(secs);
    while !cond() {
        assert!(Instant::now() < end, "{what}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// 지금 app.pid
fn app_pid(dir: &Path) -> Option<u32> {
    read_pid(&dir.join(APP_PID))
}

/// 가짜 앱 [pid]에 "crash"(표시 없이 끝남) 또는 "close"(clean_exit을 남기고 끝남)를 요청한다
fn request(dir: &Path, what: &str, pid: u32) {
    std::fs::write(dir.join(format!("{what}-{pid}")), "").unwrap();
}

/// 테스트가 직접 띄운 가짜 앱(감시보다 먼저 떠 있던 앱 등). 끝나면 바로 거둬 좀비로 남지 않게 한다 —
/// 감시는 넘겨받은 앱이 살아 있는지 PID로 본다. 돌려주는 플래그는 그 앱이 끝났는지다.
fn spawn_fake(dir: &Path, mode: &str, env: &[(&str, &str)]) -> (u32, Arc<AtomicBool>) {
    let mut command = Command::new(FAKE_APP);
    command
        .env("FAKE_DIR", dir)
        .env("FAKE_MODE", mode)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for (key, value) in env {
        command.env(key, value);
    }
    let mut child = command.spawn().unwrap();
    let pid = child.id();
    let exited = Arc::new(AtomicBool::new(false));
    let flag = exited.clone();
    std::thread::spawn(move || {
        let _ = child.wait();
        flag.store(true, Ordering::SeqCst);
    });
    (pid, exited)
}

#[test]
fn 닫는다는_표시를_남기고_끝나면_종료_코드가_1이어도_다시_띄우지_않고_감시도_끝난다() {
    let dir = fresh_dir("clean");
    let mut sup = supervisor(&dir, &[], &[("FAKE_FIRST", "clean")]);
    let status = sup.wait_exit(10, "운영자가 닫았는데 감시가 끝나지 않았다");
    assert!(status.success(), "{status:?}");
    assert_eq!(launch_args(&dir), vec!["--supervised"], "다시 띄웠다");
    assert!(log_text(&dir).contains("운영자가 앱을 닫았다"), "{}", log_text(&dir));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 표시_없이_끝나면_auto_relaunched로_다시_띄운다() {
    let dir = fresh_dir("crash");
    let mut sup = supervisor(&dir, &[], &[("FAKE_FIRST", "crash"), ("FAKE_NEXT", "clean")]);
    sup.wait_exit(15, "충돌 뒤 다시 띄운 앱이 닫혔는데 감시가 끝나지 않았다");
    assert_eq!(launch_args(&dir), vec!["--supervised", "--supervised --auto-relaunched"]);
    assert!(log_text(&dir).contains("충돌"), "{}", log_text(&dir));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 하트비트가_멈추면_강제_종료하고_다시_띄운다() {
    let dir = fresh_dir("hang");
    let mut sup = supervisor(&dir, &[], &[("FAKE_FIRST", "hang"), ("FAKE_NEXT", "clean")]);
    sup.wait_exit(20, "멈춘 앱을 다시 띄웠고 그 앱이 닫혔는데 감시가 끝나지 않았다");
    assert_eq!(launch_args(&dir), vec!["--supervised", "--supervised --auto-relaunched"]);
    assert!(log_text(&dir).contains("UI 멈춤"), "{}", log_text(&dir));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 하트비트가_한_번도_안_오면_첫_하트비트_기한_뒤_다시_띄운다() {
    let dir = fresh_dir("silent");
    let mut sup = supervisor(&dir, &[], &[("FAKE_FIRST", "silent"), ("FAKE_NEXT", "clean")]);
    sup.wait_exit(20, "하트비트 없는 앱을 다시 띄웠고 그 앱이 닫혔는데 감시가 끝나지 않았다");
    assert_eq!(launch_args(&dir), vec!["--supervised", "--supervised --auto-relaunched"]);
    assert!(log_text(&dir).contains("시작 중 멈춤"), "{}", log_text(&dir));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 운영자가_닫는_중에_멈추면_강제_종료하고_다시_띄우지_않는다() {
    let dir = fresh_dir("close_hang");
    let mut sup = supervisor(&dir, &[], &[("FAKE_FIRST", "close-hang"), ("FAKE_NEXT", "ok")]);
    let status = sup.wait_exit(15, "닫는 중 멈춘 앱을 정리하고 감시가 끝나야 한다");
    assert!(status.success(), "{status:?}");
    assert_eq!(launch_args(&dir), vec!["--supervised"], "운영자가 닫은 앱을 다시 띄웠다");
    assert!(log_text(&dir).contains("닫는 중 멈춘"), "{}", log_text(&dir));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 로그인_자동_실행이면_첫_실행에만_logon을_넘긴다() {
    let dir = fresh_dir("logon");
    let mut sup = supervisor(&dir, &["--logon"], &[("FAKE_FIRST", "crash"), ("FAKE_NEXT", "clean")]);
    sup.wait_exit(15, "감시가 끝나지 않았다");
    assert_eq!(launch_args(&dir), vec!["--supervised --logon", "--supervised --auto-relaunched"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 오디오_멈춤은_연속_3번까지만_다시_띄우고_콜백이_돌면_횟수를_초기화한다() {
    let dir = fresh_dir("audio");
    let mut sup = supervisor(&dir, &[], &[("FAKE_FIRST", "audio-stall"), ("FAKE_NEXT", "audio-stall")]);
    wait_until("오디오 멈춤 한도에 걸리지 않았다", 40, || {
        log_text(&dir).contains("더는 오디오 멈춤으로 다시 띄우지 않는다")
    });
    assert_eq!(launches(&dir).len(), 4, "첫 실행 + 다시 띄우기 3번: {}", log_text(&dir));
    assert!(log_text(&dir).contains("오디오 멈춤"), "{}", log_text(&dir));
    std::thread::sleep(Duration::from_secs(3)); // 오디오 멈춤 판단(2초)이 지나도
    assert_eq!(launches(&dir).len(), 4, "한도 뒤에도 오디오 멈춤으로 다시 띄웠다");
    assert!(sup.running(), "감시가 끝났다");

    // 장치가 돌아와 콜백이 다시 돈다
    std::fs::write(dir.join("audio-ok"), "").unwrap();
    wait_until("콜백이 다시 도는데 횟수를 초기화하지 않았다", 10, || log_text(&dir).contains("재실행 횟수를 초기화"));
    request(&dir, "close", app_pid(&dir).unwrap());
    sup.wait_exit(10, "앱을 닫았는데 감시가 끝나지 않았다");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 감시보다_먼저_떠_있던_앱을_넘겨받고_그_앱이_죽으면_다시_띄운다() {
    let dir = fresh_dir("adopt");
    let (pid, _exited) = spawn_fake(&dir, "ok", &[("FAKE_SILENT_WARNINGS", "2")]);
    wait_until("가짜 앱이 app.pid를 쓰지 않았다", 5, || app_pid(&dir) == Some(pid));
    let mut sup = supervisor(&dir, &[], &[("FAKE_NEXT", "ok")]);
    wait_until("떠 있는 앱을 넘겨받지 않았다", 10, || log_text(&dir).contains(&format!("넘겨받았다(pid {pid})")));
    wait_until("넘겨받은 앱의 무음 경고를 기록하지 않았다", 5, || log_text(&dir).contains("무음 경고"));
    assert_eq!(launches(&dir).len(), 1, "떠 있는 앱이 있는데 새로 띄웠다: {}", log_text(&dir));

    request(&dir, "crash", pid);
    wait_until("넘겨받은 앱이 죽었는데 다시 띄우지 않았다", 10, || launches(&dir).len() == 2);
    assert_eq!(launch_args(&dir)[1], "--supervised --auto-relaunched");
    wait_until("다시 띄운 앱이 app.pid를 쓰지 않았다", 5, || app_pid(&dir).is_some_and(|p| p != pid));
    request(&dir, "close", app_pid(&dir).unwrap());
    sup.wait_exit(10, "앱을 닫았는데 감시가 끝나지 않았다");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 중복_실행으로_끝나면_떠_있는_앱을_넘겨받는다() {
    let dir = fresh_dir("dup");
    let mut sup = supervisor(&dir, &[], &[("FAKE_FIRST", "dup-after-peer"), ("FAKE_NEXT", "ok")]);
    wait_until("중복 실행 뒤 떠 있는 앱을 넘겨받지 않았다", 10, || log_text(&dir).contains("넘겨받았다"));
    let peer = app_pid(&dir).unwrap();
    assert!(log_text(&dir).contains(&format!("넘겨받았다(pid {peer})")), "{}", log_text(&dir));
    assert!(log_text(&dir).contains("중복 실행"), "{}", log_text(&dir));
    std::thread::sleep(Duration::from_secs(1));
    assert_eq!(
        launches(&dir).len(),
        2,
        "중복으로 끝난 앱과 그 앱이 먼저 띄운 앱만 있어야 한다: {:?}",
        launches(&dir)
    );
    request(&dir, "close", peer);
    sup.wait_exit(10, "앱을 닫았는데 감시가 끝나지 않았다");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 감시가_이미_돌면_두_번째는_앱을_한_번_띄워_기존_창을_앞으로_가져오고_끝난다() {
    let dir = fresh_dir("second");
    let mut first = supervisor(&dir, &[], &[("FAKE_FIRST", "ok")]);
    wait_until("첫 앱이 뜨지 않았다", 10, || app_pid(&dir).is_some());
    let mut second = supervisor(&dir, &[], &[]);
    let status = second.wait_exit(5, "두 번째 감시가 바로 끝나지 않았다");
    assert!(status.success(), "{status:?}");
    wait_until("두 번째 감시가 띄운 앱이 기존 창을 앞으로 가져오지 않았다", 5, || dir.join("fronts").exists());
    assert!(first.running(), "첫 감시가 끝났다");
    assert_eq!(
        launch_args(&dir),
        vec!["--supervised".to_string(), String::new()],
        "두 번째 감시는 앱을 인자 없이 한 번만 띄운다"
    );
    request(&dir, "close", app_pid(&dir).unwrap());
    first.wait_exit(10, "앱을 닫았는데 첫 감시가 끝나지 않았다");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 죽은_앱의_pid를_다른_프로세스가_쓰고_있으면_넘겨받지_않는다() {
    let dir = fresh_dir("reuse");
    // 지난 app.pid의 PID를 앱이 아닌 프로세스가 물려받았다. app.lock은 아무도 쥐고 있지 않다.
    let (bystander, exited) = spawn_fake(&dir, "bystander", &[]);
    std::fs::write(dir.join(APP_PID), format!("{bystander}\n")).unwrap();
    std::fs::write(dir.join(APP_LOCK), "").unwrap();
    let mut sup = supervisor(&dir, &[], &[("FAKE_FIRST", "ok")]);
    wait_until("새 앱을 띄우지 않았다", 10, || app_pid(&dir).is_some_and(|p| p != bystander));
    assert!(!log_text(&dir).contains("넘겨받았다"), "앱이 아닌 프로세스를 넘겨받았다: {}", log_text(&dir));
    std::thread::sleep(Duration::from_secs(3)); // 넘겨받았다면 첫 하트비트 기한(2초)이 지나 죽였을 시간
    assert!(!exited.load(Ordering::SeqCst), "앱이 아닌 프로세스가 죽었다");
    request(&dir, "close", app_pid(&dir).unwrap());
    sup.wait_exit(10, "앱을 닫았는데 감시가 끝나지 않았다");
    request(&dir, "close", bystander);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 앱_경로를_상대_경로로_줘도_띄운다() {
    let dir = fresh_dir("relative");
    // target/에서 감시를 띄우고 앱을 "debug/fake_app"처럼 준다. 감시는 앱 폴더를 작업 폴더로 삼는다.
    let fake = Path::new(FAKE_APP);
    let base = fake.parent().unwrap().parent().unwrap();
    let relative = fake.strip_prefix(base).unwrap();
    let mut sup = supervisor_with_app(relative, Some(base), &dir, &[], &[("FAKE_FIRST", "clean")]);
    sup.wait_exit(10, "상대 경로로 준 앱이 떠서 닫혀야 감시가 끝난다");
    assert_eq!(launch_args(&dir), vec!["--supervised"], "{}", log_text(&dir));
    let _ = std::fs::remove_dir_all(dir);
}
