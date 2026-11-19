//! 장치를 다시 찾는 동안(상태는 Running, 엔진은 없음) 받은 재생 명령이 큐에 남아, 몇 시간 뒤 장치가 돌아올 때
//! 한꺼번에 나던 결함(2026-11-19 검토 세션 발견, OSC 규격 "복귀 때 한꺼번에 재생되는 일이 없게").
//! 유지자가 장치 대기·실패로 들어가는 순간 큐의 재생 명령을 재개 항목(받은 시작 위치, 그 순간부터 시간을 셈)으로
//! 옮기고 큐를 비운다. 장치가 돌아오면 기존 이어 가기(끊긴 시간 건너뛰기, ⑦ 10초 규칙)를 탄다.
//! 유지자 상태는 전역이라 이 파일의 흐름 시험은 하나다.
use rust_lib_atmos_mixer_pro::api::simple::api_play_track;
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

#[test]
fn 장치를_다시_찾는_동안_받은_재생은_대기에_들어갈_때_큐에서_재개_항목으로_옮긴다() {
    let fs = 48_000u32;
    let data = Arc::new(SoundData { samples: vec![0.1; fs as usize * 20], channels: 1, sample_rate: fs });
    GLOBAL_STATE.preloaded_sounds.write().unwrap().insert("/cue.wav".into(), data);
    *GLOBAL_STATE.config.write().unwrap() = Some(AppConfig {
        rooms: vec![RoomConfig {
            id: "r1".into(),
            name: "r1".into(),
            color_hex: "#ffffff".into(),
            volume: 1.0,
            clear_osc_address: String::new(),
            tracks: vec![TrackConfig {
                id: "t1".into(),
                name: "t1".into(),
                file_path: "/cue.wav".into(),
                volume: 1.0,
                // 메모리에 미리 읽은 단발(루프·스트리밍은 디스크에 파일이 있어야 받아진다 — 시험에는 파일이 없다).
                is_loop: false,
                is_streaming: false,
                output_channel: 0,
                output_stereo: false,
                play_osc_address: String::new(),
                stop_osc_address: String::new(),
            }],
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

    // 장치가 빠져 자기 재시작 → 찾는 중에 재생이 받아지고 → 못 찾아 대기로 간다.
    engine_keeper::report_self_restart(ENGINE_GENERATION.load(Ordering::SeqCst), Some("[ASIO] 시험 장치".into()));
    wait_until("대기 진입", || engine_keeper::state() == EngineState::WaitingForDevice);
    assert!(ACCEPTED.load(Ordering::SeqCst), "찾는 동안(Running)의 재생은 받아진다(OSC started)");

    // 큐에 남으면 장치가 돌아올 때 한꺼번에 난다 — 비어 있어야 한다.
    assert_eq!(queued_plays(), 0, "대기에 들어간 뒤에도 재생 명령이 큐에 남았다");
    // 대신 재개 항목으로 옮겨 장치가 오면 끊긴 시간을 건너뛰어 이어 간다.
    let pending = restart_resume::pending_entries();
    assert_eq!(pending.len(), 1, "재개 항목: {pending:?}");
    assert_eq!(pending[0].room_id, "r1");
    assert_eq!(pending[0].track_id, "t1");
    assert!(pending[0].seconds.abs() < 1e-9, "받은 시작 위치(0초): {}", pending[0].seconds);
    // 엔진이 없으니 재생 목록에도 남지 않는다(화면이 "재생 중"으로 보이지 않게).
    assert!(GLOBAL_STATE.playing_track_ids.read().unwrap().is_empty(), "재생 목록에 남았다");
}
