//! 이어 가지 않는 트랙(안정성 ⑦): 이어 갈 트랙이 모두 "남은 10초 이하 비반복"이라 하나도 남지 않으면 아무것도 틀지
//! 않는다 — 첫 방 테마로 시작하지 않는다(사용자 결정 2026-11-16 "아무것도 틀지 않음"). 전역 상태를 쓰므로 따로 둔다.
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::app_signals::now_ms;
use rust_lib_atmos_mixer_pro::core::show_state::{self, save, SavedTrack, ShowState};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::time::Duration;

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
        routes: vec![],
    }
}

#[test]
fn 모두_곧_끝날_트랙이면_아무것도_틀지_않고_테마도_시작하지_않는다() {
    let dir = std::env::temp_dir().join(format!("atmos_resume_tail_all_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    tone_wav(&dir.join("loop.wav"), 5.0);
    tone_wav(&dir.join("short.wav"), 15.0);
    let p = |n: &str| dir.join(format!("{n}.wav")).to_string_lossy().into_owned();
    GLOBAL_STATE.engine_sample_rate.store(48_000, Ordering::SeqCst);
    // 첫 방에 루프가 있다 — 테마로 시작하면 이 루프가 재생 명령으로 나온다.
    *GLOBAL_STATE.config.write().unwrap() = Some(AppConfig {
        rooms: vec![RoomConfig {
            id: "r".into(),
            name: "r".into(),
            color_hex: "#ffffff".into(),
            volume: 1.0,
            clear_osc_address: String::new(),
            tracks: vec![track("loopA", &p("loop"), true), track("sfxShort", &p("short"), false)],
        }],
        ..AppConfig::default()
    });
    GLOBAL_STATE.clear_playing_tracks();
    while GLOBAL_STATE.command_receiver.try_recv().is_ok() {}

    save(
        &dir,
        &ShowState {
            saved_at_ms: now_ms() - 2_000,
            active_room_id: Some("r".into()),
            tracks: vec![SavedTrack { room_id: "r".into(), track_id: "sfxShort".into(), seconds: 9.0 }],
        },
    )
    .unwrap();
    show_state::start(dir.clone(), Duration::from_secs(3600));

    show_state::resume(now_ms()).unwrap();
    let mut plays = Vec::new();
    while let Ok(c) = GLOBAL_STATE.command_receiver.try_recv() {
        if let AudioCommand::PlayTrack { instance, .. } = c {
            plays.push(instance.track_id_str.clone());
        }
    }
    assert!(plays.is_empty(), "남은 6초 비반복만 있었는데 재생 명령이 나갔다(테마 시작 포함): {plays:?}");
    assert!(GLOBAL_STATE.playing_track_ids.read().unwrap().is_empty());

    let _ = std::fs::remove_dir_all(&dir);
}
