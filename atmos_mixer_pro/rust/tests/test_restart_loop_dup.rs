//! 짧은 자기 재시작 중 루프 두 벌 막기(2026-11-19 Main 배분).
//! 자기 재시작 때 take_snapshot이 재생 목록을 비우고 재생 중이던 루프를 재개 대기로 옮긴다. 그 사이 같은 루프를 다시
//! 누르면 예전에는 재생 쪽 루프 중복 막기(재생 목록만 봄)가 못 보고 받아 줘, 재개 때 같은 루프가 서로 다른 위상의 두
//! 벌로 났다. 이제 재개 대기에 있는 루프도 이미 재생 중으로 보고 건너뛴다(응답은 지금 중복 막기와 같은 Ok).
//! 전역 상태를 쓰므로 시험은 하나다.
use rust_lib_atmos_mixer_pro::api::simple::api_play_track;
use rust_lib_atmos_mixer_pro::audio::engine::ENGINE_GENERATION;
use rust_lib_atmos_mixer_pro::audio::playback_cursor::CURSOR_TABLE;
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::restart_resume;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;

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
        routes: vec![],
    }
}

fn drain_plays() -> Vec<String> {
    let mut v = Vec::new();
    while let Ok(c) = GLOBAL_STATE.command_receiver.try_recv() {
        if let AudioCommand::PlayTrack { instance, .. } = c {
            v.push(instance.track_id_str.clone());
        }
    }
    v
}

fn tone_wav(path: &std::path::Path) {
    let spec = hound::WavSpec { channels: 1, sample_rate: 48_000, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for i in 0..48_000 {
        w.write_sample(((i % 100) as i16 - 50) * 100).unwrap();
    }
    w.finalize().unwrap();
}

#[test]
fn 재개를_기다리는_루프를_다시_눌러도_두_벌로_틀지_않는다() {
    let dir = std::env::temp_dir().join(format!("atmos_loop_dup_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("bgm.wav");
    tone_wav(&path);
    let p = path.to_string_lossy().to_string();

    let mut config = AppConfig::default();
    config.rooms.push(RoomConfig {
        id: "r1".into(),
        name: "R1".into(),
        color_hex: "#ffffff".into(),
        volume: 1.0,
        clear_osc_address: String::new(),
        tracks: vec![track("bgm", &p, true), track("bgm_other", &p, true)],
    });
    *GLOBAL_STATE.config.write().unwrap() = Some(config);
    GLOBAL_STATE.clear_playing_tracks();
    CURSOR_TABLE.clear_all();
    restart_resume::cancel_all();
    drain_plays();

    // 루프 bgm이 1.5초에서 돌던 중 엔진이 스스로 재시작했다: 스냅샷이 재개 대기로 옮기고 재생 목록을 비운다.
    GLOBAL_STATE.add_playing_track(201, "bgm".into());
    CURSOR_TABLE.publish(0, 201, 1.5);
    restart_resume::take_snapshot(ENGINE_GENERATION.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!(restart_resume::pending_entries().len(), 1, "스냅샷이 루프를 재개 대기에 넣지 않았다");

    // 재개를 기다리는 사이 같은 루프를 다시 누른다(OSC·화면).
    assert!(api_play_track("r1".into(), "bgm".into()).is_ok(), "중복 막기는 오류가 아니라 Ok(OSC started)다");
    let queued = drain_plays();
    assert!(
        !queued.iter().any(|t| t == "bgm"),
        "재개를 기다리는 루프를 다시 받아 큐에 넣었다(재개 때 두 벌): {queued:?}"
    );
    assert_eq!(restart_resume::pending_entries().len(), 1, "재개 대기가 바뀌었다");

    // 재개 대기에 없는 다른 루프는 그대로 틀린다.
    assert!(api_play_track("r1".into(), "bgm_other".into()).is_ok());
    assert_eq!(drain_plays(), vec!["bgm_other".to_string()], "다른 루프까지 막았다");

    restart_resume::cancel_all();
    GLOBAL_STATE.clear_playing_tracks();
    let _ = std::fs::remove_dir_all(&dir);
}
