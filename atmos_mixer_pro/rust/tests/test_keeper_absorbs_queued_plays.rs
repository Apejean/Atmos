//! 장치를 다시 찾는 동안(상태는 Running, 엔진은 없음) 받은 재생 명령이 큐에 남아, 몇 시간 뒤 장치가 돌아올 때
//! 한꺼번에 나던 결함(2026-11-19 검토 세션 발견, OSC 규격 "복귀 때 한꺼번에 재생되는 일이 없게").
//! 유지자가 장치 대기·실패로 들어가는 순간 큐의 재생 명령을 재개 항목(받은 시작 위치, 그 순간부터 시간을 셈)으로
//! 옮기고 큐를 비운다. 장치가 오면 기존 이어 가기(끊긴 시간 건너뛰기, ⑦ 10초 규칙)를 탄다. 받았다가 곧 멈춘
//! 재생은 옮기지 않는다(재생 목록에 남은 것만 — take_snapshot과 같은 기준). 이미 재개 대기에 있는 루프를 다시 누른
//! 것은 옮기지 않는다(루프 중복 막기와 같음 — 두 벌이 서로 다른 위상으로 나지 않게).
//! 유지자 상태는 전역이라 이 파일의 흐름 시험은 하나다.
use rust_lib_atmos_mixer_pro::api::simple::{api_play_track, api_stop_track};
use rust_lib_atmos_mixer_pro::audio::engine::ENGINE_GENERATION;
use rust_lib_atmos_mixer_pro::audio::player::SoundData;
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::engine_keeper::{self, EngineState, OpenRequest, StartFailure};
use rust_lib_atmos_mixer_pro::core::restart_resume;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

static ACCEPTED: AtomicBool = AtomicBool::new(false);

/// 자기 재시작의 장치 찾기 중에 OSC 재생이 들어온다(그때 상태는 Running). 장치는 없다.
fn fake_open(req: &OpenRequest) -> Result<(), StartFailure> {
    if req.self_restart {
        ACCEPTED.store(api_play_track("r1".into(), "t1".into()).is_ok(), Ordering::SeqCst);
        // 받았다가 곧 멈춘 재생(큐에는 "재생 t2 → 정지 t2"가 함께 있다). 장치가 돌아와도 살아나면 안 된다.
        let _ = api_play_track("r1".into(), "t2".into());
        let _ = api_stop_track("r1".into(), "t2".into());
        // 끊기기 전부터 돌던 루프를 다시 누름(스냅샷이 재생 목록을 비워 중복 막기가 못 본다 — 받아진다).
        let _ = api_play_track("r1".into(), "lp".into());
    }
    Err(StartFailure::NotFound("없음".into()))
}

fn fake_resume() {}

fn wait_until(what: &str, mut cond: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !cond() {
        assert!(Instant::now() < deadline, "시간 안에 되지 않았다: {what} (상태 {:?})", engine_keeper::state());
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn queued_plays() -> usize {
    let mut n = 0;
    while let Ok(cmd) = GLOBAL_STATE.command_receiver.try_recv() {
        if matches!(cmd, AudioCommand::PlayTrack { .. }) {
            n += 1;
        }
    }
    n
}

fn tone_wav(path: &std::path::Path, seconds: f32) {
    let spec = hound::WavSpec { channels: 1, sample_rate: 48_000, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for i in 0..(48_000.0 * seconds) as usize {
        let v = (i as f32 * 200.0 * std::f32::consts::TAU / 48_000.0).sin() * 3000.0;
        w.write_sample(v as i16).unwrap();
    }
    w.finalize().unwrap();
}

/// 메모리에 미리 읽은 단발(루프·스트리밍은 디스크에 파일이 있어야 받아진다 — 시험에는 파일이 없다).
fn one_shot(id: &str) -> TrackConfig {
    TrackConfig {
        id: id.into(),
        name: id.into(),
        file_path: "/cue.wav".into(),
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
fn 장치를_다시_찾는_동안_받은_재생은_대기에_들어갈_때_큐에서_재개_항목으로_옮긴다() {
    let fs = 48_000u32;
    let data = Arc::new(SoundData { samples: vec![0.1; fs as usize * 20], channels: 1, sample_rate: fs });
    GLOBAL_STATE.preloaded_sounds.write().unwrap().insert("/cue.wav".into(), data);
    let dir = std::env::temp_dir().join(format!("absorb_queue_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let loop_path = dir.join("loop.wav");
    tone_wav(&loop_path, 4.0);
    let loop_path = loop_path.to_string_lossy().to_string();
    *GLOBAL_STATE.config.write().unwrap() = Some(AppConfig {
        rooms: vec![RoomConfig {
            id: "r1".into(),
            name: "r1".into(),
            color_hex: "#ffffff".into(),
            volume: 1.0,
            clear_osc_address: String::new(),
            tracks: vec![one_shot("t1"), one_shot("t2"), looped("lp", &loop_path)],
        }],
        ..AppConfig::default()
    });
    engine_keeper::set_test_hooks(fake_open, fake_resume, 5);

    // 엔진이 돌던 중(Running). 큐·재생 목록·재개 대기는 비어 있다.
    engine_keeper::user_started(Some("[ASIO] 시험 장치".into()), &Ok(()));
    assert_eq!(engine_keeper::state(), EngineState::Running);
    while GLOBAL_STATE.command_receiver.try_recv().is_ok() {}
    GLOBAL_STATE.clear_playing_tracks();
    restart_resume::cancel_all();
    // 루프 lp가 돌던 중에 끊긴다: 엔진 스레드는 재시작 직전 스냅샷을 떠(lp가 재개 대기로) 재생 목록·큐를 비운 뒤 알린다.
    api_play_track("r1".into(), "lp".into()).expect("루프 재생");
    restart_resume::take_snapshot(ENGINE_GENERATION.load(Ordering::SeqCst));
    assert_eq!(restart_resume::pending_entries().len(), 1, "스냅샷의 lp");

    // 장치가 빠져 자기 재시작 → 찾는 중에 재생이 받아지고 → 못 찾아 대기로 간다.
    engine_keeper::report_self_restart(ENGINE_GENERATION.load(Ordering::SeqCst), Some("[ASIO] 시험 장치".into()));
    wait_until("대기 진입", || engine_keeper::state() == EngineState::WaitingForDevice);
    assert!(ACCEPTED.load(Ordering::SeqCst), "찾는 동안(Running)의 재생은 받아진다(OSC started)");
    // 유지자는 대기 상태를 먼저 알린 뒤(그 뒤 재생은 거절) 큐를 옮긴다 — 옮기기가 끝나기를 기다린다.
    wait_until("큐를 재개 항목으로 옮김", || restart_resume::pending_entries().iter().any(|e| e.track_id == "t1"));

    // 큐에 남으면 장치가 돌아올 때 한꺼번에 난다 — 비어 있어야 한다.
    assert_eq!(queued_plays(), 0, "대기에 들어간 뒤에도 재생 명령이 큐에 남았다");
    // 대신 재개 항목으로 옮겨 장치가 오면 끊긴 시간을 건너뛰어 이어 간다. 멈춘 t2는 옮기지 않는다.
    let pending = restart_resume::pending_entries();
    let count = |id: &str| pending.iter().filter(|e| e.track_id == id).count();
    assert_eq!(count("t1"), 1, "받은 단발 t1: {pending:?}");
    assert_eq!(count("t2"), 0, "멈춘 t2는 살아나면 안 된다: {pending:?}");
    assert_eq!(count("lp"), 1, "이미 재개 대기에 있는 루프는 한 벌만: {pending:?}");
    let t1 = pending.iter().find(|e| e.track_id == "t1").unwrap();
    assert_eq!(t1.room_id, "r1");
    assert!(t1.seconds.abs() < 1e-9, "받은 시작 위치(0초): {}", t1.seconds);
    // 엔진이 없으니 재생 목록에도 남지 않는다(화면이 "재생 중"으로 보이지 않게).
    assert!(GLOBAL_STATE.playing_track_ids.read().unwrap().is_empty(), "재생 목록에 남았다");
    let _ = std::fs::remove_dir_all(dir);
}

fn looped(id: &str, path: &str) -> TrackConfig {
    TrackConfig { id: id.into(), name: id.into(), file_path: path.into(), is_loop: true, ..one_shot(id) }
}
