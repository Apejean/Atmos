//! 무인 운영 중 시스템이 잠들지 않게 한다. 화면은 전원 설정대로 꺼져도 된다(사용자 결정, 2026-10-05).
//! 앱이 켜져 있는 동안 유지되고, 앱이 끝나면 함께 풀린다.
use std::sync::Once;

static START: Once = Once::new();

/// 앱 시작 때 부른다(`api_init_app`). 여러 번 불러도 한 번만 건다.
pub fn prevent_system_sleep() {
    START.call_once(start);
}

#[cfg(target_os = "windows")]
fn start() {
    // ES_CONTINUOUS로 건 상태는 건 스레드에 묶이므로, 끝나지 않는 전용 스레드에서 걸고 잠들어 둔다.
    let _ = std::thread::Builder::new().name("keep-awake".into()).spawn(|| {
        use windows::Win32::System::Power::{
            SetThreadExecutionState, ES_CONTINUOUS, ES_SYSTEM_REQUIRED,
        };
        unsafe {
            SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED);
        }
        loop {
            std::thread::park();
        }
    });
}

#[cfg(target_os = "macos")]
fn start() {
    // caffeinate -i: 유휴 시스템 절전 방지, -w: 이 프로세스가 끝나면 caffeinate도 끝난다.
    let _ = std::process::Command::new("/usr/bin/caffeinate")
        .args(["-i", "-w", &std::process::id().to_string()])
        .spawn();
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn start() {}
