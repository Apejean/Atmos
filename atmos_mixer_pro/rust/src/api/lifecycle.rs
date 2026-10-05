//! 앱 수명 신호: 시작 관문, 하트비트, 정상 종료 표시(docs/superpowers/specs/2026-10-05-crash-relaunch-supervisor-design.md
//! 4.2·4.3). 판단은 core::app_signals에 있다.
use crate::core::app_signals::{self, Gate};
use crate::core::log_file::log_dir;
use crate::core::state::GLOBAL_STATE;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

/// 시작 관문의 결정. Dart `main`이 창을 띄우기 전에 받아 그대로 한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupDecision {
    /// 계속 시작한다.
    Proceed,
    /// 감시 프로그램을 띄웠다. 앱은 종료 코드 0으로 끝낸다(감시가 앱을 다시 띄운다).
    HandedToSupervisor,
    /// 다른 앱이 이미 떠 있다(그 창을 앞으로 가져왔다). 종료 코드 3으로 끝낸다.
    Duplicate,
}

/// 앱이 끝날 때까지 쥐는 app.lock.
static APP_LOCK_FILE: Mutex<Option<File>> = Mutex::new(None);

/// 창을 띄우기 전에 한 번 부른다. [args]는 앱 실행 인자다.
pub fn api_startup_gate(args: Vec<String>) -> StartupDecision {
    let dir = log_dir();
    let supervisor = supervisor_next_to_app();
    let gate = Gate {
        dir: &dir,
        supervisor_exe: supervisor.as_deref(),
        pid: std::process::id(),
        launch_supervisor: &launch_supervisor,
        bring_to_front: &bring_window_to_front,
    };
    let (decision, lock) = app_signals::startup_gate(&args, &gate);
    if decision == StartupDecision::Proceed && lock.is_none() {
        GLOBAL_STATE.log("시작 관문: app.lock을 잡지 못했다 — 중복 실행을 막지 못한 채 진행한다".into());
    }
    *APP_LOCK_FILE.lock().unwrap_or_else(|e| e.into_inner()) = lock;
    GLOBAL_STATE.log(format!("시작 관문: {decision:?} (인자 {args:?})"));
    decision
}

/// Dart가 2초마다 부른다. 감시가 이 파일로 앱이 응답하는지 본다.
pub fn api_heartbeat() {
    let _ = app_signals::write_heartbeat(&log_dir(), &app_signals::current_heartbeat());
}

/// 운영자가 창을 닫을 때 엔진을 멈추기 전에 부른다. 감시는 이 앱을 다시 띄우지 않는다.
pub fn api_mark_clean_exit() {
    if let Err(e) = app_signals::mark_clean_exit(&log_dir(), std::process::id()) {
        GLOBAL_STATE.log(format!("정상 종료 표시를 남기지 못했다(감시가 다시 띄울 수 있다): {e}"));
    }
}

fn supervisor_next_to_app() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?.parent()?.join(app_signals::SUPERVISOR_EXE);
    exe.is_file().then_some(exe)
}

/// 감시 프로그램을 띄운다. 이 앱은 곧 끝나므로 기다리지 않는다.
#[allow(clippy::zombie_processes)]
fn launch_supervisor(exe: &Path) -> std::io::Result<()> {
    Command::new(exe)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
}

/// 그 PID의 보이는 최상위 창을 앞으로 가져온다(최소화돼 있으면 되살린다).
#[cfg(target_os = "windows")]
fn bring_window_to_front(pid: u32) {
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsIconic, IsWindowVisible, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };

    struct Search {
        pid: u32,
        found: Option<HWND>,
    }

    unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let search = &mut *(lparam.0 as *mut Search);
        let mut owner = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut owner as *mut u32));
        if owner == search.pid && IsWindowVisible(hwnd).as_bool() {
            search.found = Some(hwnd);
            return BOOL(0); // 찾았으니 그만 돈다
        }
        BOOL(1)
    }

    let mut search = Search { pid, found: None };
    unsafe {
        let _ = EnumWindows(Some(visit), LPARAM(&mut search as *mut Search as isize));
        if let Some(hwnd) = search.found {
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

/// macOS는 확인용이라 창을 앞으로 가져오지 않는다(현장은 Windows).
#[cfg(not(target_os = "windows"))]
fn bring_window_to_front(_pid: u32) {}
