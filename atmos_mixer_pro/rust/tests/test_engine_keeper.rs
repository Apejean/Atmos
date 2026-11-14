//! 엔진 유지자(core::engine_keeper, 안정성 ③). 실제 장치 대신 흉내 열기 함수와 짧은 간격(단위 5ms)으로
//! 대기 → 복귀, 자기 재시작, 열기 연속 실패, 사용자 정지, 시작 멈춤, 대기 중 재생 거절을 본다.
//! 예전에는 설정 장치·기본 장치가 모두 실패하면 아무도 다시 시도하지 않았다(무음으로 남음).
//! 유지자 상태는 전역이라 흐름 시험은 하나로 차례대로 돈다. 순수 함수 시험은 따로 둔다.
use rust_lib_atmos_mixer_pro::api::simple::api_play_track;
use rust_lib_atmos_mixer_pro::audio::engine::ENGINE_GENERATION;
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::app_signals::current_heartbeat;
use rust_lib_atmos_mixer_pro::core::engine_keeper::{
    self, retry_delay_units, status_string, EngineState, FailCause, KeeperCore, OpenRequest, StartFailure,
};
use rust_lib_atmos_mixer_pro::core::restart_resume::{skip_outage, ResumeEntry};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use rust_lib_atmos_mixer_pro::osc::reply::{reason, reason_for_play_error};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

static SCRIPT: Mutex<VecDeque<Result<(), StartFailure>>> = Mutex::new(VecDeque::new());
static CALLS: AtomicU32 = AtomicU32::new(0);
static SELF_RESTART_CALLS: AtomicU32 = AtomicU32::new(0);
static RESUMES: AtomicU32 = AtomicU32::new(0);

/// 대본대로 결과를 돌려준다. 대본이 비면 "장치 없음".
fn fake_open(req: &OpenRequest) -> Result<(), StartFailure> {
    CALLS.fetch_add(1, Ordering::SeqCst);
    if req.self_restart {
        SELF_RESTART_CALLS.fetch_add(1, Ordering::SeqCst);
    }
    SCRIPT
        .lock()
        .unwrap()
        .pop_front()
        .unwrap_or_else(|| Err(StartFailure::NotFound("없음".into())))
}

fn fake_resume() {
    RESUMES.fetch_add(1, Ordering::SeqCst);
}

fn script(results: Vec<Result<(), StartFailure>>) {
    let mut s = SCRIPT.lock().unwrap();
    s.clear();
    s.extend(results);
}

fn not_found() -> Result<(), StartFailure> {
    Err(StartFailure::NotFound("없음".into()))
}

fn open_failed() -> Result<(), StartFailure> {
    Err(StartFailure::OpenFailed("열기 실패".into()))
}

fn wait_until(what: &str, mut cond: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !cond() {
        assert!(Instant::now() < deadline, "시간 안에 되지 않았다: {what} (상태 {:?})", engine_keeper::state());
        std::thread::sleep(Duration::from_millis(2));
    }
}

const DEV: &str = "[ASIO] 시험 장치";

fn dev() -> Option<String> {
    Some(DEV.to_string())
}

fn track(id: &str) -> TrackConfig {
    TrackConfig {
        id: id.into(),
        name: id.into(),
        file_path: "/없는/파일.wav".into(),
        volume: 1.0,
        is_loop: false,
        is_streaming: false,
        output_channel: 0,
        output_stereo: false,
        play_osc_address: String::new(),
        stop_osc_address: String::new(),
    }
}

#[test]
fn 유지자는_장치를_계속_찾고_돌아오면_이어_가며_정지와_멈춤을_지킨다() {
    engine_keeper::set_test_hooks(fake_open, fake_resume, 5);
    *GLOBAL_STATE.config.write().unwrap() = Some(AppConfig {
        rooms: vec![RoomConfig {
            id: "r1".into(),
            name: "r1".into(),
            color_hex: "#ffffff".into(),
            volume: 1.0,
            clear_osc_address: String::new(),
            tracks: vec![track("t1")],
        }],
        ..AppConfig::default()
    });

    // 1) 사용자 시작이 "장치 없음"으로 실패 → 대기. 대기 중 재생은 쌓지 않고 버린다(D11). 대본이 비어 있어 계속 없다.
    script(vec![]);
    engine_keeper::user_started(dev(), &not_found());
    assert_eq!(engine_keeper::state(), EngineState::WaitingForDevice);
    assert!(!engine_keeper::accepts_play());
    while GLOBAL_STATE.command_receiver.try_recv().is_ok() {}
    let err = api_play_track("r1".into(), "t1".into()).expect_err("대기 중 재생이 받아들여졌다");
    assert_eq!(reason_for_play_error(&err.message), reason::ENGINE_WAITING, "OSC 거절 이유: {}", err.message);
    assert!(GLOBAL_STATE.command_receiver.try_recv().is_err(), "대기 중 재생 명령이 큐에 쌓였다");
    let hb = current_heartbeat().render();
    assert!(hb.contains("engine_state=WaitingForDevice\n"), "하트비트: {hb}");
    assert!(hb.contains("engine_progress="), "하트비트: {hb}");

    wait_until("대기 중 재시도", || CALLS.load(Ordering::SeqCst) >= 2);
    assert_eq!(engine_keeper::state(), EngineState::WaitingForDevice, "포기하지 않고 대기에 머문다");

    // 2) 한 번 더 없다가 다음에 열린다 → Running, 재개 한 번.
    let progress_before = engine_keeper::progress();
    CALLS.store(0, Ordering::SeqCst);
    script(vec![not_found(), Ok(())]);
    wait_until("장치 복귀", || engine_keeper::state() == EngineState::Running);
    assert!(CALLS.load(Ordering::SeqCst) >= 2, "재시도 횟수 {}", CALLS.load(Ordering::SeqCst));
    // 상태가 Running이 된 뒤 열기 잠금을 놓고 재개한다 — 재개는 조금 늦게 온다.
    wait_until("다시 연 뒤 재개", || RESUMES.load(Ordering::SeqCst) == 1);
    assert!(engine_keeper::progress() >= progress_before + 2, "진행 카운터가 시도마다 늘어야 한다");
    assert!(engine_keeper::accepts_play());

    // 3) 자기 재시작: 첫 시도(자기 재시작, 30초 찾기)가 실패하면 대기로 가고, 재시도로 돌아온다.
    CALLS.store(0, Ordering::SeqCst);
    script(vec![not_found(), Ok(())]);
    engine_keeper::report_self_restart(ENGINE_GENERATION.load(Ordering::SeqCst), dev());
    wait_until("자기 재시작 뒤 복귀", || {
        engine_keeper::state() == EngineState::Running && CALLS.load(Ordering::SeqCst) == 2
    });
    assert_eq!(SELF_RESTART_CALLS.load(Ordering::SeqCst), 1, "자기 재시작 시도는 한 번이고 나머지는 재시도다");
    wait_until("자기 재시작 뒤 재개", || RESUMES.load(Ordering::SeqCst) == 2);

    // 4) 장치는 보이는데 열기가 3번 연속 실패 → Failed(open_failed). 그래도 계속 시도해 회복한다(U3).
    CALLS.store(0, Ordering::SeqCst);
    script(vec![open_failed(), open_failed(), Ok(())]);
    engine_keeper::user_started(dev(), &open_failed());
    wait_until("열기 실패 3번", || engine_keeper::state() == EngineState::Failed(FailCause::OpenFailed));
    assert!(!engine_keeper::accepts_play(), "실패 중에도 재생을 버린다");
    wait_until("열기 실패 뒤 회복", || engine_keeper::state() == EngineState::Running);
    assert_eq!(CALLS.load(Ordering::SeqCst), 3);

    // 5) 대기 중 사용자 정지 → StoppedByUser, 더 시도하지 않는다.
    script(vec![]);
    engine_keeper::user_started(dev(), &not_found());
    wait_until("대기 중 재시도", || CALLS.load(Ordering::SeqCst) >= 4);
    engine_keeper::user_stopped();
    assert_eq!(engine_keeper::state(), EngineState::StoppedByUser);
    std::thread::sleep(Duration::from_millis(30)); // 진행 중이던 시도가 끝날 시간
    let calls = CALLS.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(300)); // 간격 표의 몇 단계(5ms 단위)를 넘는 시간
    assert_eq!(CALLS.load(Ordering::SeqCst), calls, "사용자 정지 뒤에도 다시 시도했다");
    assert_eq!(engine_keeper::state(), EngineState::StoppedByUser);
    assert!(engine_keeper::accepts_play(), "사용자 정지는 재생을 버리지 않는다(지금 동작)");

    // 6) 시작 호출이 돌아오지 않음 → Failed(start_hung), 더 시도하지 않는다(마지막에 둔다: 이 뒤로는 계속 멈춤 상태).
    engine_keeper::user_started(dev(), &Err(StartFailure::Hung));
    assert_eq!(engine_keeper::state(), EngineState::Failed(FailCause::StartHung));
    assert!(engine_keeper::is_hung());
    let calls = CALLS.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(CALLS.load(Ordering::SeqCst), calls, "멈춘 뒤에도 다시 시도했다");
    let hb = current_heartbeat().render();
    assert!(hb.contains("engine_state=Failed\n"), "하트비트: {hb}");
}

#[test]
fn 재시도_간격은_1_2_4_8_16_30초이고_그_뒤_30초다() {
    let delays: Vec<u64> = (1..=8).map(retry_delay_units).collect();
    assert_eq!(delays, vec![1, 2, 4, 8, 16, 30, 30, 30]);
}

#[test]
fn 상태_기계는_실패_종류에_따라_대기와_실패를_가른다() {
    let mut core = KeeperCore::default();
    assert_eq!(core.state, EngineState::StoppedByUser);
    assert_eq!(core.on_failed(&StartFailure::NotFound(String::new())), Some(1));
    assert_eq!(core.state, EngineState::WaitingForDevice);
    assert_eq!(core.on_failed(&StartFailure::OpenFailed(String::new())), Some(2));
    assert_eq!(core.on_failed(&StartFailure::OpenFailed(String::new())), Some(4));
    assert_eq!(core.state, EngineState::WaitingForDevice, "열기 실패 2번은 아직 대기");
    assert_eq!(core.on_failed(&StartFailure::OpenFailed(String::new())), Some(8));
    assert_eq!(core.state, EngineState::Failed(FailCause::OpenFailed), "열기 실패 3번이면 실패");
    assert_eq!(core.on_failed(&StartFailure::NotFound(String::new())), Some(16));
    assert_eq!(core.state, EngineState::WaitingForDevice, "장치가 사라지면 다시 대기(연속이 끊김)");
    core.on_started();
    assert_eq!((core.state, core.attempts, core.open_failures), (EngineState::Running, 0, 0));
    assert_eq!(core.on_failed(&StartFailure::Unavailable(String::new())), None);
    assert_eq!(core.state, EngineState::Failed(FailCause::Unavailable));
    assert_eq!(core.on_failed(&StartFailure::Hung), None);
    assert_eq!(core.state, EngineState::Failed(FailCause::StartHung));
}

#[test]
fn 상태_방송_문자열은_대기와_실패를_구분한다() {
    let mut core = KeeperCore::default();
    core.on_failed(&StartFailure::NotFound(String::new()));
    core.on_failed(&StartFailure::NotFound(String::new()));
    assert_eq!(
        status_string(&core, Some(Duration::from_secs(2))).as_deref(),
        Some("WaitingForDevice:2:2")
    );
    core.on_failed(&StartFailure::Hung);
    assert_eq!(status_string(&core, None).as_deref(), Some("EngineDown:start_hung"));
    core.on_started();
    assert_eq!(status_string(&core, None), None);
}

#[test]
fn 끊긴_시간만큼_건너뛴다() {
    let e = |id: &str, s: f64| ResumeEntry { room_id: "r".into(), track_id: id.into(), seconds: s };
    let info = |id: &str| match id {
        "loop10" => Some((true, 10.0)),
        "sfx10" => Some((false, 10.0)),
        _ => None,
    };
    let out = skip_outage(
        vec![(e("loop10", 8.0), 5.0), (e("sfx10", 3.0), 2.5), (e("sfx10", 9.0), 2.0), (e("unknown", 4.0), 3.0)],
        info,
    );
    assert_eq!(out.len(), 3, "끝을 넘은 비반복은 빠진다: {out:?}");
    assert!((out[0].seconds - 3.0).abs() < 1e-9, "반복은 길이의 나머지: {out:?}");
    assert!((out[1].seconds - 5.5).abs() < 1e-9, "비반복은 그대로 더한다: {out:?}");
    assert!((out[2].seconds - 4.0).abs() < 1e-9, "길이를 모르면 건너뛰지 않는다: {out:?}");
    // 같은 끊김을 겪은 같은 길이의 두 트랙은 상대 위치가 그대로다(그룹 동기).
    let pair = skip_outage(vec![(e("loop10", 2.0), 9.0), (e("loop10", 2.0), 9.0)], info);
    assert_eq!(pair[0].seconds, pair[1].seconds);
}
