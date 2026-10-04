//! 재시작 복원(core::restart_resume): 스냅샷·취소·재개. 전역 상태를 쓰므로 테스트는 하나다.
use rust_lib_atmos_mixer_pro::api::simple::{
    api_clear_room, api_get_playback_positions, api_set_active_room, api_stop_all, api_stop_track,
    build_play_track_command,
};
use rust_lib_atmos_mixer_pro::audio::engine::ENGINE_GENERATION;
use rust_lib_atmos_mixer_pro::audio::playback_cursor::CURSOR_TABLE;
use rust_lib_atmos_mixer_pro::audio::player::SoundData;
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::restart_resume::{self, ResumeEntry};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn track(id: &str, path: &str) -> TrackConfig {
    TrackConfig {
        id: id.into(),
        name: id.into(),
        file_path: path.into(),
        volume: 1.0,
        is_loop: false,
        is_streaming: false,
        output_channel: 1,
        output_stereo: false,
        play_osc_address: String::new(),
        stop_osc_address: String::new(),
    }
}

fn drain() -> Vec<AudioCommand> {
    let mut v = Vec::new();
    while let Ok(c) = GLOBAL_STATE.command_receiver.try_recv() {
        v.push(c);
    }
    v
}

/// 재생 명령들의 (트랙, 시작 위치 초)
fn plays(cmds: &[AudioCommand]) -> Vec<(String, f64)> {
    cmds.iter()
        .filter_map(|c| match c {
            AudioCommand::PlayTrack { instance, .. } => {
                Some((instance.track_id_str.clone(), instance.position_seconds()))
            }
            _ => None,
        })
        .collect()
}

/// 감시 루프가 넘기는 엔진 세대(테스트에는 엔진이 없어 바뀌지 않는다)
fn gen() -> u64 {
    ENGINE_GENERATION.load(std::sync::atomic::Ordering::SeqCst)
}

fn entry(track: &str, seconds: f64) -> ResumeEntry {
    ResumeEntry { room_id: "r1".into(), track_id: track.into(), seconds }
}

#[test]
fn 스냅샷을_떠서_멈춘_위치부터_재개하고_사용자_정지와_재시작_폭주를_처리한다() {
    let fs = 48_000u32;
    let data = Arc::new(SoundData { samples: vec![0.1; fs as usize * 20], channels: 1, sample_rate: fs });
    GLOBAL_STATE.preloaded_sounds.write().unwrap().insert("/sfx.wav".into(), data.clone());
    GLOBAL_STATE.preloaded_sounds.write().unwrap().insert("/bgm2.wav".into(), data.clone());
    let mut config = AppConfig::default();
    config.rooms.push(RoomConfig {
        id: "r1".into(),
        name: "R1".into(),
        color_hex: "#ffffff".into(),
        volume: 1.0,
        volume_osc_address: String::new(),
        clear_osc_address: String::new(),
        tracks: vec![track("sfx", "/sfx.wav"), track("bgm2", "/bgm2.wav")],
    });
    *GLOBAL_STATE.config.write().unwrap() = Some(config);
    GLOBAL_STATE.clear_playing_tracks();
    CURSOR_TABLE.clear_all();
    drain();

    // 재생 중: sfx 두 인스턴스(3.0초, 7.5초), bgm2(2.0초), 설정에 없는 트랙
    for (inst, id, secs, slot) in [(101u64, "sfx", 3.0, 0usize), (102, "sfx", 7.5, 1), (103, "bgm2", 2.0, 2), (104, "gone", 1.0, 3)] {
        GLOBAL_STATE.add_playing_track(inst, id.into());
        CURSOR_TABLE.publish(slot, inst, secs);
    }
    // 끝났는데 목록 정리만 늦은 sfx(위치 없음, 큐에도 없음)와, 아직 옛 믹서가 가져가지 않은 bgm2 재생 명령
    GLOBAL_STATE.add_playing_track(105, "sfx".into());
    GLOBAL_STATE.add_playing_track(106, "bgm2".into());
    GLOBAL_STATE
        .command_sender
        .send(build_play_track_command(106, 1, 1, "bgm2".into(), Some(data), None, fs, 1, false, 1.0, 1.0, 1, false, None))
        .unwrap();
    GLOBAL_STATE.command_sender.send(AudioCommand::StopAll).unwrap(); // 옛 엔진용 미처리 명령

    // 1) 스냅샷: 같은 트랙 두 인스턴스는 각자 위치로, 설정에서 사라진 트랙은 빠진다(Focus 3·4).
    //    큐에 남은 재생은 0초부터, 끝난 인스턴스는 다시 틀지 않는다.
    restart_resume::take_snapshot(gen());
    assert_eq!(
        restart_resume::pending_entries(),
        vec![entry("bgm2", 0.0), entry("bgm2", 2.0), entry("sfx", 3.0), entry("sfx", 7.5)]
    );
    assert!(GLOBAL_STATE.playing_track_ids.read().unwrap().is_empty(), "옛 인스턴스가 재생 목록에 남았다");
    // 커서 표는 옛 콜백이 drop 전까지 계속 쓰므로 스냅샷이 지우지 않는다(새 믹서가 지운다). 조회에는 안 보여야 한다.
    assert!(api_get_playback_positions().is_empty(), "옛 위치가 조회에 보인다");
    assert!(drain().is_empty(), "옛 엔진용 명령이 큐에 남았다");

    // 2) 재개 전에 또 재시작(재생 중인 것 없음): 대기열을 잃지 않는다(Focus 2)
    restart_resume::take_snapshot(gen());
    assert_eq!(restart_resume::pending_entries().len(), 4);

    // 3) 기다리는 사이 사용자가 sfx를 정지하면 sfx는 재개하지 않는다(Focus 1)
    api_stop_track("r1".into(), "sfx".into()).unwrap();
    restart_resume::resume_pending();
    let started = plays(&drain());
    assert_eq!(started.len(), 2, "재개된 재생: {started:?}");
    assert!(started.iter().all(|(t, _)| t == "bgm2"), "재개된 재생: {started:?}");
    assert!((started[0].1 - 0.0).abs() < 1e-6 && (started[1].1 - 2.0).abs() < 1e-6, "재개 위치 {started:?}");
    assert!(restart_resume::pending_entries().is_empty());
    assert!(GLOBAL_STATE.playing_track_ids.read().unwrap().values().any(|t| t == "bgm2"), "재개한 트랙이 재생 목록에 없다");

    // 4) 아무것도 재생 중이 아닐 때 재시작해도 아무것도 틀지 않는다(Focus 5)
    GLOBAL_STATE.clear_playing_tracks();
    CURSOR_TABLE.clear_all();
    restart_resume::take_snapshot(gen());
    restart_resume::resume_pending();
    assert!(plays(&drain()).is_empty());

    // 4-1) 그사이 사용자가 엔진을 직접 멈추거나 다시 띄웠으면(세대가 다르면) 스냅샷은 아무것도 하지 않는다
    GLOBAL_STATE.add_playing_track(151, "bgm2".into());
    CURSOR_TABLE.publish(9, 151, 1.5);
    GLOBAL_STATE.command_sender.send(AudioCommand::StopAll).unwrap();
    restart_resume::take_snapshot(gen() + 1);
    assert!(restart_resume::pending_entries().is_empty(), "세대가 바뀌었는데 재개 대기가 생겼다");
    assert_eq!(GLOBAL_STATE.playing_track_ids.read().unwrap().len(), 1, "세대가 바뀌었는데 재생 목록을 지웠다");
    assert_eq!(drain().len(), 1, "세대가 바뀌었는데 큐를 비웠다");
    GLOBAL_STATE.clear_playing_tracks();

    // 5) 전체 정지는 대기열을 비운다(Focus 1)
    GLOBAL_STATE.add_playing_track(201, "bgm2".into());
    CURSOR_TABLE.publish(10, 201, 1.0);
    restart_resume::take_snapshot(gen());
    assert_eq!(restart_resume::pending_entries().len(), 1);
    api_stop_all().unwrap();
    assert!(restart_resume::pending_entries().is_empty());

    // 6) 방 비우기(대시보드·OSC 룸 전환)는 그 방의 대기만 버린다(Focus 1)
    GLOBAL_STATE.add_playing_track(301, "bgm2".into());
    CURSOR_TABLE.publish(11, 301, 4.0);
    restart_resume::take_snapshot(gen());
    restart_resume::cancel_room("r2");
    assert_eq!(restart_resume::pending_entries(), vec![entry("bgm2", 4.0)], "다른 방을 비웠는데 대기가 사라졌다");
    api_set_active_room(Some("r1".into())).unwrap();
    api_clear_room("r1".into()).unwrap();
    assert!(restart_resume::pending_entries().is_empty(), "방을 비웠는데 재개 대기가 남았다");
    drain();

    // 7) 재동기화 완료 신호: 오면 바로, 안 오면 시간 초과
    let seq = restart_resume::RESTART_SEQ.load(std::sync::atomic::Ordering::SeqCst) + 1;
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(30));
        restart_resume::ack(seq);
    });
    let t0 = Instant::now();
    assert!(restart_resume::wait_for_ack(seq, Duration::from_secs(2)));
    assert!(t0.elapsed() < Duration::from_millis(500));
    assert!(!restart_resume::wait_for_ack(seq + 1, Duration::from_millis(100)));
}
