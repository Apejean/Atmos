//! 감시 프로그램(supervisor/)이 읽는 신호 파일과 시작 관문(스펙 4.2·4.3).
//! 파일 이름과 하트비트 형식은 감시 크레이트(supervisor/src/heartbeat.rs)와 같아야 한다.
//! 오디오 스레드는 건드리지 않는다 — 이미 있는 원자 값을 읽기만 한다(DSP 3법칙).
use crate::api::lifecycle::StartupDecision;
use crate::core::state::GLOBAL_STATE;
use std::fs::{File, OpenOptions, TryLockError};
use std::path::Path;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

pub const APP_LOCK: &str = "app.lock";
pub const APP_PID: &str = "app.pid";
pub const HEARTBEAT: &str = "heartbeat";
pub const CLEAN_EXIT: &str = "clean_exit";
pub const SUPERVISOR_LOCK: &str = "supervisor.lock";
/// 앱 옆에 이 감시 프로그램이 있으면 직접 실행된 앱을 감시로 넘긴다(배포는 Windows만, macOS 번들에는 없다).
#[cfg(target_os = "windows")]
pub const SUPERVISOR_EXE: &str = "atmos_supervisor.exe";
#[cfg(not(target_os = "windows"))]
pub const SUPERVISOR_EXE: &str = "atmos_supervisor";

/// 재생 중 무음 경고 누적 횟수. core::show_state가 올리고 하트비트가 감시에 알린다.
pub static SILENT_WARNINGS: AtomicU32 = AtomicU32::new(0);

/// 벽시계 밀리초.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 임시 파일에 쓰고 이름을 바꾼다. 읽는 쪽(감시, 다음 실행)이 반쯤 쓴 내용을 보지 않는다.
pub fn atomic_write(path: &Path, contents: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}

/// 감시가 읽는 하트비트 값.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeartbeatValues {
    pub pid: u32,
    /// UI가 하트비트를 부른 시각(밀리초). 부를 때마다 커진다.
    pub ui_ms: u64,
    /// 마지막 오디오 콜백 시각(GLOBAL_STATE.watchdog_last_callback, 밀리초).
    pub audio_cb_ms: u64,
    pub engine_active: bool,
    pub silent_warnings: u32,
}

impl HeartbeatValues {
    pub fn render(&self) -> String {
        format!(
            "pid={}\nui_ms={}\naudio_cb_ms={}\nengine_active={}\nsilent_warnings={}\n",
            self.pid,
            self.ui_ms,
            self.audio_cb_ms,
            u8::from(self.engine_active),
            self.silent_warnings
        )
    }
}

/// 지금 값으로 하트비트를 만든다. 같은 밀리초에 두 번 불러도 ui_ms가 커진다 — 감시는 값이 바뀌는지만 본다.
pub fn current_heartbeat() -> HeartbeatValues {
    static LAST_UI_MS: AtomicU64 = AtomicU64::new(0);
    let now = now_ms();
    let prev = LAST_UI_MS
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |prev| Some(now.max(prev + 1)))
        .unwrap_or_else(|prev| prev);
    HeartbeatValues {
        pid: std::process::id(),
        ui_ms: now.max(prev + 1),
        audio_cb_ms: GLOBAL_STATE.watchdog_last_callback.load(Ordering::Relaxed),
        engine_active: crate::api::simple::ENGINE_ACTIVE.load(Ordering::SeqCst),
        silent_warnings: SILENT_WARNINGS.load(Ordering::Relaxed),
    }
}

pub fn write_heartbeat(dir: &Path, values: &HeartbeatValues) -> std::io::Result<()> {
    atomic_write(&dir.join(HEARTBEAT), &values.render())
}

/// 운영자가 창을 닫는다는 표시. 감시는 이 PID의 앱이 끝나면 다시 띄우지 않는다.
pub fn mark_clean_exit(dir: &Path, pid: u32) -> std::io::Result<()> {
    atomic_write(&dir.join(CLEAN_EXIT), &format!("pid={pid}\n"))
}

/// 시작 관문이 쓰는 폴더와 바깥 동작. 테스트는 동작을 바꿔 끼운다.
pub struct Gate<'a> {
    /// 신호 파일 폴더(앱 로그 폴더).
    pub dir: &'a Path,
    /// 앱 옆의 감시 프로그램(있을 때만).
    pub supervisor_exe: Option<&'a Path>,
    /// 이 앱의 PID.
    pub pid: u32,
    pub launch_supervisor: &'a dyn Fn(&Path) -> std::io::Result<()>,
    pub bring_to_front: &'a dyn Fn(u32),
}

/// 시작 관문(스펙 4.3). `Proceed`면 app.lock 파일을 함께 돌려준다 — 앱이 끝날 때까지 쥐고 있어야 다른
/// 실행이 중복으로 끝난다. 잠금 파일을 열 수조차 없으면 잠금 없이 진행한다(중복 막기보다 공연이 먼저다).
pub fn startup_gate(args: &[String], gate: &Gate) -> (StartupDecision, Option<File>) {
    let supervised = args.iter().any(|a| a == "--supervised");
    // 감시 없이 직접 실행됐고, 감시 프로그램이 옆에 있는데 돌고 있지 않으면 감시로 넘긴다.
    let hand_over_to = gate
        .supervisor_exe
        .filter(|_| !supervised && !lock_held(&gate.dir.join(SUPERVISOR_LOCK)));
    let lock = match open_and_lock(&gate.dir.join(APP_LOCK)) {
        Ok(Some(file)) => Some(file),
        Ok(None) => {
            // 다른 앱이 떠 있다. 그 창을 앞으로 가져오고, 감시가 없으면 감시를 띄워 그 앱을 넘겨받게 한다.
            if let Some(pid) = read_pid(&gate.dir.join(APP_PID)) {
                (gate.bring_to_front)(pid);
            }
            if let Some(exe) = hand_over_to {
                let _ = (gate.launch_supervisor)(exe);
            }
            return (StartupDecision::Duplicate, None);
        }
        Err(_) => None,
    };
    if let Some(exe) = hand_over_to {
        // 잠금을 쥔 채 감시를 띄우고 돌아가며 놓는다. 감시가 띄우는 앱은 그보다 한참 늦게 잠금을 잡는다.
        match (gate.launch_supervisor)(exe) {
            Ok(()) => return (StartupDecision::HandedToSupervisor, None),
            Err(e) => GLOBAL_STATE.log(format!("감시 프로그램을 띄우지 못했다 — 감시 없이 진행한다: {e}")),
        }
    }
    if lock.is_some() {
        let _ = atomic_write(&gate.dir.join(APP_PID), &format!("{}\n", gate.pid));
    }
    let _ = std::fs::remove_file(gate.dir.join(CLEAN_EXIT));
    (StartupDecision::Proceed, lock)
}

/// [path]를 열어 잠근다. 다른 프로세스(또는 같은 프로세스의 다른 열기)가 쥐고 있으면 Ok(None).
fn open_and_lock(path: &Path) -> std::io::Result<Option<File>> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let file = OpenOptions::new().create(true).truncate(false).read(true).write(true).open(path)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(e)) => Err(e),
    }
}

/// 누가 [path] 잠금을 쥐고 있나(잠깐 잡았다 놓는다).
fn lock_held(path: &Path) -> bool {
    matches!(open_and_lock(path), Ok(None))
}

fn read_pid(path: &Path) -> Option<u32> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}
