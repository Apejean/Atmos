//! 장치를 기다리는 동안의 공연 이어 가기(core::show_state + core::engine_keeper). 2026-11-16 ④ 실기 F2:
//! 인터페이스를 끈 채 로그인 자동 실행으로 켜면 이어 가기가 대기 중에 거절돼 사라지고, 공연 상태 파일이 빈 목록으로
//! 덮여 장치가 돌아와도 아무것도 나지 않았다. 사용자 결정(2026-11-16): 이어 갈 목록을 들고 있다가 장치가 열리면
//! 저장된 위치부터 튼다(기다린 시간은 건너뛰지 않는다). 전역 상태를 쓰므로 테스트는 하나다.
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::app_signals::now_ms;
use rust_lib_atmos_mixer_pro::core::engine_keeper::{self, EngineState, OpenRequest, StartFailure};
use rust_lib_atmos_mixer_pro::core::restart_resume;
use rust_lib_atmos_mixer_pro::core::show_state::{self, capture, load, save, SavedTrack, ShowState};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::time::{Duration, Instant};

static SCRIPT: Mutex<VecDeque<Result<(), StartFailure>>> = Mutex::new(VecDeque::new());

/// 대본대로 결과를 돌려준다. 대본이 비면 "장치 없음".
fn fake_open(_req: &OpenRequest) -> Result<(), StartFailure> {
    SCRIPT.lock().unwrap().pop_front().unwrap_or_else(|| Err(StartFailure::NotFound("없음".into())))
}

fn fake_resume() {}

fn tone_wav(path: &std::path::Path, seconds: f32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 48_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for i in 0..(48_000.0 * seconds) as usize {
        let v = (i as f32 * 200.0 * std::f32::consts::TAU / 48_000.0).sin() * 3000.0;
        w.write_sample(v as i16).unwrap();
    }
    w.finalize().unwrap();
}

fn track(id: &str, path: &str, is_loop: bool) -> TrackConfig {
    TrackConfig {
        id: id.into(),
        name: id.into(),
        file_path: path.into(),
        volume: 1.0,
        is_loop,
        is_streaming: false,
        output_channel: 1,
        output_stereo: false,
        play_osc_address: String::new(),
        stop_osc_address: String::new(),
    }
}

fn room(id: &str, tracks: Vec<TrackConfig>) -> RoomConfig {
    RoomConfig {
        id: id.into(),
        name: id.into(),
        color_hex: "#ffffff".into(),
        volume: 1.0,
        clear_osc_address: String::new(),
        tracks,
    }
}

fn drain() -> Vec<AudioCommand> {
    let mut v = Vec::new();
    while let Ok(c) = GLOBAL_STATE.command_receiver.try_recv() {
        v.push(c);
    }
    v
}

/// 재생 명령들의 (트랙, 시작 위치 초), 트랙 이름 순
fn plays(cmds: &[AudioCommand]) -> Vec<(String, f64)> {
    let mut v: Vec<(String, f64)> = cmds
        .iter()
        .filter_map(|c| match c {
            AudioCommand::PlayTrack { instance, .. } => Some((instance.track_id_str.clone(), instance.position_seconds())),
            _ => None,
        })
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

fn saved(room: &str, track: &str, seconds: f64) -> SavedTrack {
    SavedTrack { room_id: room.into(), track_id: track.into(), seconds }
}

fn wait_until(what: &str, mut cond: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !cond() {
        assert!(Instant::now() < deadline, "시간 안에 되지 않았다: {what} (상태 {:?})", engine_keeper::state());
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn 장치를_기다리는_동안_이어_가기를_들고_있다가_장치가_열리면_저장된_위치부터_튼다() {
    let dir = std::env::temp_dir().join(format!("atmos_show_resume_waiting_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // 길이를 다르게 둔다. 길이가 같은 트랙은 그룹 시작이 가장 앞선 위치로 맞춘다(group_start::align_plan).
    tone_wav(&dir.join("la.wav"), 4.0);
    tone_wav(&dir.join("lb.wav"), 5.0);
    let p = |n: &str| dir.join(format!("{n}.wav")).to_string_lossy().into_owned();
    GLOBAL_STATE.engine_sample_rate.store(48_000, Ordering::SeqCst);
    *GLOBAL_STATE.config.write().unwrap() = Some(AppConfig {
        rooms: vec![room("a", vec![track("la", &p("la"), true)]), room("b", vec![track("lb", &p("lb"), true)])],
        ..AppConfig::default()
    });
    GLOBAL_STATE.clear_playing_tracks();
    drain();

    // 단위를 길게(100ms) 두어 이 시험 동안 유지자가 스스로 다시 열지 않게 한다(대본이 비어 있으면 계속 "없음").
    engine_keeper::set_test_hooks(fake_open, fake_resume, 100);
    engine_keeper::user_started(Some("[ASIO] 시험 장치".into()), &Err(StartFailure::NotFound("없음".into())));
    assert_eq!(engine_keeper::state(), EngineState::WaitingForDevice);

    // 앱을 켤 때 읽어 둘 지난 공연 상태(로그인 자동 실행 전에 강제 종료됨).
    let previous = ShowState {
        saved_at_ms: now_ms() - 3_000,
        active_room_id: Some("b".into()),
        tracks: vec![saved("a", "la", 1.5), saved("b", "lb", 2.5)],
    };
    save(&dir, &previous).unwrap();
    show_state::start(dir.clone(), Duration::from_millis(100));

    // 1) 대기 중 이어 가기: 지금 틀지 않고(재생 명령 없음, 거절도 아님) 들고 있는다.
    show_state::resume(now_ms()).expect("대기 중 이어 가기가 실패로 끝났다");
    assert!(plays(&drain()).is_empty(), "장치가 없는데 재생 명령을 보냈다");

    // 2) 들고 있는 동안 공연 상태 파일을 빈 목록으로 덮지 않는다(앱이 다시 켜져도 이어 갈 수 있게).
    let held = capture(now_ms());
    assert_eq!(held.active_room_id, Some("b".into()), "들고 있는 이어 가기의 활성 방을 저장해야 한다");
    assert_eq!(held.tracks, vec![saved("a", "la", 1.5), saved("b", "lb", 2.5)]);
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(
        load(&dir).unwrap().map(|s| s.tracks),
        Some(vec![saved("a", "la", 1.5), saved("b", "lb", 2.5)]),
        "기다리는 동안 공연 상태 파일이 빈 목록으로 덮였다"
    );

    // 3) 장치가 돌아오면(유지자가 다시 엶) 저장된 위치부터 한 그룹으로 튼다. 기다린 시간은 건너뛰지 않는다.
    SCRIPT.lock().unwrap().push_back(Ok(()));
    wait_until("장치 복귀", || engine_keeper::state() == EngineState::Running);
    let mut started = Vec::new();
    wait_until("들고 있던 이어 가기 재생", || {
        started.extend(plays(&drain()));
        started.len() >= 2
    });
    started.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(started.len(), 2, "이어 튼 트랙: {started:?}");
    assert_eq!(started[0].0, "la");
    assert!((started[0].1 - 1.5).abs() < 1e-6, "la 시작 위치 {}", started[0].1);
    assert_eq!(started[1].0, "lb");
    assert!((started[1].1 - 2.5).abs() < 1e-6, "lb 시작 위치 {}", started[1].1);
    assert_eq!(*GLOBAL_STATE.active_room_id.read().unwrap(), Some("b".to_string()));

    // 4) 대기 중 사용자가 정지하면 들고 있던 것(첫 방 테마로 시작하는 계획 포함)은 버린다.
    GLOBAL_STATE.clear_playing_tracks();
    drain();
    engine_keeper::user_started(Some("[ASIO] 시험 장치".into()), &Err(StartFailure::NotFound("없음".into())));
    assert_eq!(engine_keeper::state(), EngineState::WaitingForDevice);
    show_state::resume(now_ms()).expect("두 번째 이어 가기(첫 방 테마로 시작)가 실패로 끝났다");
    assert!(plays(&drain()).is_empty());
    restart_resume::cancel_all();
    SCRIPT.lock().unwrap().push_back(Ok(()));
    wait_until("장치 복귀", || engine_keeper::state() == EngineState::Running);
    std::thread::sleep(Duration::from_millis(300));
    assert!(plays(&drain()).is_empty(), "정지 뒤에 들고 있던 이어 가기를 틀었다");

    drain();
    let _ = std::fs::remove_dir_all(&dir);
}
