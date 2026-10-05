//! 첫 방 테마 시작(api::show::api_theme_start). 감시가 다시 띄운 앱이 이어 갈 수 없을 때와 OSC 테마 시작이
//! 이 함수를 쓴다. 전역 상태를 쓰므로 테스트는 하나다.
use rust_lib_atmos_mixer_pro::api::show::api_theme_start;
use rust_lib_atmos_mixer_pro::api::simple::{api_play_track, api_set_active_room};
use rust_lib_atmos_mixer_pro::audio::player::SoundData;
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Arc;

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
        volume_osc_address: String::new(),
        clear_osc_address: String::new(),
        tracks,
    }
}

fn playing() -> Vec<String> {
    let mut v: Vec<String> = GLOBAL_STATE.playing_track_ids.read().unwrap().values().cloned().collect();
    v.sort();
    v
}

fn drain() -> Vec<AudioCommand> {
    let mut v = Vec::new();
    while let Ok(c) = GLOBAL_STATE.command_receiver.try_recv() {
        v.push(c);
    }
    v
}

#[test]
fn 테마_시작은_전체_정지_뒤_첫_방을_활성으로_하고_그_방_루프만_튼다() {
    let dir = std::env::temp_dir().join(format!("atmos_theme_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for name in ["la", "lb"] {
        tone_wav(&dir.join(format!("{name}.wav")), 3.0);
    }
    let p = |n: &str| dir.join(format!("{n}.wav")).to_string_lossy().into_owned();
    GLOBAL_STATE.sound_cache.write().unwrap().insert(
        "/oa.wav".into(),
        Arc::new(SoundData { samples: vec![0.1; 48_000], channels: 1, sample_rate: 48_000 }),
    );
    GLOBAL_STATE.engine_sample_rate.store(48_000, Ordering::SeqCst);
    *GLOBAL_STATE.config.write().unwrap() = Some(AppConfig {
        rooms: vec![
            room("a", vec![track("la", &p("la"), true), track("oa", "/oa.wav", false)]),
            room("b", vec![track("lb", &p("lb"), true)]),
        ],
        ..AppConfig::default()
    });
    GLOBAL_STATE.clear_playing_tracks();

    // 방 B BGM과 방 A 단발이 도는 중이다
    api_set_active_room(Some("b".into())).unwrap();
    api_play_track("b".into(), "lb".into()).unwrap();
    api_play_track("a".into(), "oa".into()).unwrap();
    drain();

    api_theme_start().unwrap();
    assert_eq!(*GLOBAL_STATE.active_room_id.read().unwrap(), Some("a".to_string()));
    assert_eq!(playing(), vec!["la".to_string()], "첫 방 루프만 재생 목록에 남아야 한다");
    let cmds = drain();
    let stop_all_at = cmds.iter().position(|c| matches!(c, AudioCommand::StopAll)).expect("전체 정지 명령이 없다");
    let plays: Vec<(usize, String)> = cmds
        .iter()
        .enumerate()
        .filter_map(|(i, c)| match c {
            AudioCommand::PlayTrack { instance, .. } => Some((i, instance.track_id_str.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(plays.iter().map(|(_, t)| t.as_str()).collect::<Vec<_>>(), vec!["la"]);
    assert!(plays[0].0 > stop_all_at, "재생 명령이 전체 정지보다 먼저 나갔다");

    // 방이 없으면 정지만 한다
    *GLOBAL_STATE.config.write().unwrap() = Some(AppConfig::default());
    api_theme_start().unwrap();
    assert!(playing().is_empty());
    assert_eq!(*GLOBAL_STATE.active_room_id.read().unwrap(), None);

    drain();
    let _ = std::fs::remove_dir_all(&dir);
}
