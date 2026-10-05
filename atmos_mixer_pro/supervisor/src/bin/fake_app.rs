//! 감시 프로그램 테스트용 가짜 앱. 앱처럼 app.lock을 쥐고 app.pid·heartbeat·clean_exit을 남긴다.
//! 배포에는 넣지 않는다(CI는 atmos_supervisor만 복사한다).
//!
//! 동작은 환경 변수로 정한다. FAKE_MODE가 있으면 그것을 쓴다. 없으면 --supervised가 없을 때 "front"
//! (앱 시작 관문의 중복 실행 흉내), --auto-relaunched면 FAKE_NEXT, 아니면 FAKE_FIRST다(기본 "ok").
//! 실행할 때마다 FAKE_DIR/launches에 "PID 인자…"를 남긴다. FAKE_DIR/crash-<PID>가 생기면 표시 없이 1로,
//! close-<PID>가 생기면 clean_exit을 남기고 0으로 끝난다. 테스트가 실패해 남더라도 2분 뒤에는 스스로 끝난다.
use atmos_supervisor::heartbeat::{atomic_write, read_pid, Heartbeat, APP_LOCK, APP_PID, CLEAN_EXIT, HEARTBEAT};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{exit, Command, Stdio};
use std::time::{Duration, Instant};

/// 하트비트 간격(앱은 2초, 테스트는 짧게).
const BEAT: Duration = Duration::from_millis(100);
/// crash·clean·hang·close-hang 모드가 정상으로 도는 시간.
const BRIEF: Duration = Duration::from_millis(300);
/// 테스트가 실패해 아무도 닫지 않아도 이 시간 뒤에는 끝난다.
const MAX_LIFE: Duration = Duration::from_secs(120);

fn append(path: &Path, line: &str) {
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
    }
}

fn mark_clean_exit(dir: &Path, pid: u32) {
    let _ = atomic_write(&dir.join(CLEAN_EXIT), &format!("pid={pid}\n"));
}

/// 다른 앱을 먼저 띄워 두고 그 앱이 app.pid를 쓸 때까지 기다린다. 이 가짜 앱이 곧 끝나므로
/// 먼저 띄운 앱은 init이 거둔다(기다리지 않는다).
#[allow(clippy::zombie_processes)]
fn spawn_peer(dir: &Path) {
    let peer = Command::new(std::env::current_exe().expect("현재 exe"))
        .env("FAKE_MODE", "ok")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("먼저 띄울 앱");
    let end = Instant::now() + Duration::from_secs(5);
    while read_pid(&dir.join(APP_PID)) != Some(peer.id()) && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn main() {
    let dir = PathBuf::from(std::env::var("FAKE_DIR").expect("FAKE_DIR"));
    let args: Vec<String> = std::env::args().skip(1).collect();
    let pid = std::process::id();
    append(&dir.join("launches"), &format!("{pid} {}", args.join(" ")));
    let has = |flag: &str| args.iter().any(|a| a == flag);
    let mode = match std::env::var("FAKE_MODE") {
        Ok(mode) => mode,
        Err(_) if !has("--supervised") => "front".to_string(),
        Err(_) => {
            let key = if has("--auto-relaunched") { "FAKE_NEXT" } else { "FAKE_FIRST" };
            std::env::var(key).unwrap_or_else(|_| "ok".to_string())
        }
    };
    let start = Instant::now();

    match mode.as_str() {
        // 앱의 시작 관문: 이미 떠 있으면 기존 창을 앞으로 가져오고 3으로 끝난다
        "front" => {
            append(&dir.join("fronts"), &pid.to_string());
            exit(3)
        }
        "dup" => exit(3),
        // 다른 앱을 먼저 띄워 두고 중복으로 끝난다(감시가 그 앱을 넘겨받아야 한다)
        "dup-after-peer" => {
            spawn_peer(&dir);
            exit(3)
        }
        // 앱이 아닌 다른 프로세스: 잠금도 신호도 없이 닫으라고 할 때까지 산다
        "bystander" => loop {
            if dir.join(format!("close-{pid}")).exists() || start.elapsed() > MAX_LIFE {
                exit(0)
            }
            std::thread::sleep(BEAT);
        },
        _ => {}
    }

    // 앱처럼 잠금을 쥐고 PID를 쓴다. 이미 떠 있으면 중복이다.
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dir.join(APP_LOCK))
        .expect("app.lock");
    if lock.try_lock().is_err() {
        exit(3)
    }
    atomic_write(&dir.join(APP_PID), &format!("{pid}\n")).expect("app.pid");
    let silent_warnings = std::env::var("FAKE_SILENT_WARNINGS").ok().and_then(|v| v.parse().ok()).unwrap_or(0);

    let (mut ui, mut audio) = (0u64, 1000u64);
    let mut marked = false;
    loop {
        if dir.join(format!("crash-{pid}")).exists() {
            exit(1)
        }
        if dir.join(format!("close-{pid}")).exists() {
            mark_clean_exit(&dir, pid);
            exit(0)
        }
        if start.elapsed() > MAX_LIFE {
            exit(0)
        }
        let brief_over = start.elapsed() >= BRIEF;
        let beating = match mode.as_str() {
            "crash" if brief_over => exit(1),
            "clean" if brief_over => {
                mark_clean_exit(&dir, pid);
                exit(1)
            }
            // 운영자가 닫았는데(clean_exit) 닫는 중에 멈췄다
            "close-hang" if brief_over => {
                if !marked {
                    mark_clean_exit(&dir, pid);
                    marked = true;
                }
                false
            }
            "hang" if brief_over => false,
            "silent" => false,
            _ => true,
        };
        if beating {
            ui += 1;
            if mode != "audio-stall" || dir.join("audio-ok").exists() {
                audio += 1;
            }
            let beat = Heartbeat { pid, ui_ms: ui, audio_cb_ms: audio, engine_active: true, silent_warnings };
            let _ = atomic_write(&dir.join(HEARTBEAT), &beat.render());
        }
        std::thread::sleep(BEAT);
    }
}
