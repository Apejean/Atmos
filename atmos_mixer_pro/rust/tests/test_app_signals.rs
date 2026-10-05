//! 앱이 감시 프로그램에 남기는 신호와 시작 관문(core::app_signals). 실제 로그 폴더 대신 임시 폴더를 쓴다.
use rust_lib_atmos_mixer_pro::api::lifecycle::StartupDecision;
use rust_lib_atmos_mixer_pro::core::app_signals::{
    current_heartbeat, mark_clean_exit, startup_gate, write_heartbeat, Gate, APP_LOCK, APP_PID, CLEAN_EXIT,
    HEARTBEAT, SILENT_WARNINGS, SUPERVISOR_LOCK,
};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

fn fresh_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("atmos_signals_test_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// 다른 프로세스처럼 [path] 잠금을 쥔다(같은 프로세스라도 따로 연 파일끼리는 잠금이 부딪힌다).
fn hold_lock(path: &Path) -> File {
    let file = OpenOptions::new().create(true).truncate(false).read(true).write(true).open(path).unwrap();
    file.try_lock().unwrap();
    file
}

/// 관문을 한 번 돌린 결과
struct GateRun {
    decision: StartupDecision,
    lock: Option<File>,
    launched: Vec<PathBuf>,
    fronted: Vec<u32>,
}

fn run_gate(dir: &Path, supervisor_exe: Option<&Path>, argv: &[&str]) -> GateRun {
    let launched = RefCell::new(Vec::new());
    let fronted = RefCell::new(Vec::new());
    let args: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    let (decision, lock) = {
        let launch = |exe: &Path| -> std::io::Result<()> {
            launched.borrow_mut().push(exe.to_path_buf());
            Ok(())
        };
        let front = |pid: u32| fronted.borrow_mut().push(pid);
        let gate = Gate { dir, supervisor_exe, pid: 4242, launch_supervisor: &launch, bring_to_front: &front };
        startup_gate(&args, &gate)
    };
    GateRun { decision, lock, launched: launched.into_inner(), fronted: fronted.into_inner() }
}

#[test]
fn 직접_실행했고_감시가_옆에_있고_돌지_않으면_감시로_넘기고_잠금을_놓는다() {
    let dir = fresh_dir("handoff");
    let exe = dir.join("atmos_supervisor");
    let run = run_gate(&dir, Some(&exe), &[]);
    assert_eq!(run.decision, StartupDecision::HandedToSupervisor);
    assert_eq!(run.launched, vec![exe]);
    assert!(run.lock.is_none());
    assert!(!dir.join(APP_PID).exists(), "넘기는 앱이 app.pid를 쓰면 감시가 끝나 가는 앱을 넘겨받는다");
    drop(hold_lock(&dir.join(APP_LOCK))); // 감시가 띄울 앱이 잠금을 잡을 수 있다
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 감시가_없거나_감시가_띄웠거나_감시가_돌면_진행하고_잠금과_pid를_쥔다() {
    // 감시 exe가 없다(개발 빌드·macOS)
    let dir = fresh_dir("proceed");
    std::fs::write(dir.join(CLEAN_EXIT), "pid=1\n").unwrap();
    let run = run_gate(&dir, None, &[]);
    assert_eq!(run.decision, StartupDecision::Proceed);
    assert!(run.lock.is_some(), "app.lock을 쥐어야 한다");
    assert!(run.launched.is_empty());
    assert_eq!(std::fs::read_to_string(dir.join(APP_PID)).unwrap().trim(), "4242");
    assert!(!dir.join(CLEAN_EXIT).exists(), "지난 clean_exit이 남아 있다");
    drop(run.lock);

    // 감시가 띄웠다(--supervised): 감시 exe가 있어도 넘기지 않는다
    let exe = dir.join("atmos_supervisor");
    let run = run_gate(&dir, Some(&exe), &["--supervised", "--auto-relaunched"]);
    assert_eq!(run.decision, StartupDecision::Proceed);
    assert!(run.launched.is_empty());
    drop(run.lock);

    // 감시가 이미 돈다(supervisor.lock이 잡혀 있다): 직접 실행이어도 넘기지 않는다(그 감시가 넘겨받는다)
    let supervisor = hold_lock(&dir.join(SUPERVISOR_LOCK));
    let run = run_gate(&dir, Some(&exe), &[]);
    assert_eq!(run.decision, StartupDecision::Proceed);
    assert!(run.launched.is_empty());
    drop(run.lock);
    drop(supervisor);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 앱이_이미_떠_있으면_그_창을_앞으로_가져오고_중복으로_끝난다() {
    let dir = fresh_dir("dup");
    let first = run_gate(&dir, None, &["--supervised"]);
    assert_eq!(first.decision, StartupDecision::Proceed);
    std::fs::write(dir.join(APP_PID), "777\n").unwrap(); // 떠 있는 앱의 PID

    let second = run_gate(&dir, None, &[]);
    assert_eq!(second.decision, StartupDecision::Duplicate);
    assert_eq!(second.fronted, vec![777]);
    assert!(second.launched.is_empty(), "감시 exe가 없으면 감시를 띄우지 않는다");
    assert_eq!(std::fs::read_to_string(dir.join(APP_PID)).unwrap().trim(), "777", "중복 실행이 app.pid를 덮어썼다");

    // 감시 없이 떠 있는 앱 + 감시 exe 있음: 감시를 띄워 그 앱을 넘겨받게 한다
    let exe = dir.join("atmos_supervisor");
    let third = run_gate(&dir, Some(&exe), &[]);
    assert_eq!(third.decision, StartupDecision::Duplicate);
    assert_eq!(third.launched, vec![exe]);
    drop(first.lock);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 하트비트에는_다섯_값이_들어가고_부를_때마다_ui_ms가_커진다() {
    GLOBAL_STATE.watchdog_last_callback.store(777, Ordering::Relaxed);
    SILENT_WARNINGS.store(2, Ordering::Relaxed);
    let a = current_heartbeat();
    let b = current_heartbeat();
    assert_eq!(a.pid, std::process::id());
    assert_eq!(a.audio_cb_ms, 777);
    assert!(!a.engine_active, "테스트에는 엔진이 없다");
    assert_eq!(a.silent_warnings, 2);
    assert!(b.ui_ms > a.ui_ms, "같은 밀리초에 불러도 ui_ms가 바뀌어야 감시가 응답으로 본다");

    let dir = fresh_dir("hb");
    write_heartbeat(&dir, &b).unwrap();
    let text = std::fs::read_to_string(dir.join(HEARTBEAT)).unwrap();
    let kv: HashMap<&str, &str> = text.lines().filter_map(|l| l.split_once('=')).collect();
    assert_eq!(kv.len(), 5, "감시가 읽는 키는 다섯 개다: {text}");
    assert_eq!(kv["pid"], b.pid.to_string());
    assert_eq!(kv["ui_ms"], b.ui_ms.to_string());
    assert_eq!(kv["audio_cb_ms"], "777");
    assert_eq!(kv["engine_active"], "0");
    assert_eq!(kv["silent_warnings"], "2");
    assert!(!dir.join(format!("{HEARTBEAT}.tmp")).exists(), "임시 파일이 남았다");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 정상_종료_표시는_닫는_앱의_pid다() {
    let dir = fresh_dir("clean");
    mark_clean_exit(&dir, 4242).unwrap();
    assert_eq!(std::fs::read_to_string(dir.join(CLEAN_EXIT)).unwrap(), "pid=4242\n");
    let _ = std::fs::remove_dir_all(dir);
}
