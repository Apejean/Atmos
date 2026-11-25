//! 이어 가지 않는 트랙(안정성 ⑦, 사용자 결정 2026-11-10): 앱 재실행·로그인 이어 가기(core::show_state)에서 무한루프가
//! 아닌 트랙 중 남은 길이가 10초 이하인 것만 버리고 나머지는 이어 간다. 파일 길이는 실제 WAV 머리에서 읽는다.
//! 전역 상태를 쓰므로 테스트는 하나다(모두 버려지는 경우는 test_resume_tail_all_dropped.rs).
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

fn drain_plays() -> Vec<(String, f64)> {
    let mut v = Vec::new();
    while let Ok(c) = GLOBAL_STATE.command_receiver.try_recv() {
        if let AudioCommand::PlayTrack { instance, .. } = c {
            v.push((instance.track_id_str.clone(), instance.position_seconds()));
        }
    }
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

#[test]
fn 남은_10초_이하_비반복만_버리고_나머지는_저장된_위치부터_이어_간다() {
    let dir = std::env::temp_dir().join(format!("atmos_resume_tail_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    tone_wav(&dir.join("loop.wav"), 5.0);
    tone_wav(&dir.join("short.wav"), 15.0);
    tone_wav(&dir.join("long.wav"), 30.0);
    let p = |n: &str| dir.join(format!("{n}.wav")).to_string_lossy().into_owned();
    GLOBAL_STATE.engine_sample_rate.store(48_000, Ordering::SeqCst);
    *GLOBAL_STATE.config.write().unwrap() = Some(AppConfig {
        rooms: vec![RoomConfig {
            id: "r".into(),
            name: "r".into(),
            color_hex: "#ffffff".into(),
            volume: 1.0,
            clear_osc_address: String::new(),
            tracks: vec![
                track("loopA", &p("loop"), true),
                track("sfxShort", &p("short"), false),
                track("sfxLong", &p("long"), false),
            ],
        }],
        ..AppConfig::default()
    });
    GLOBAL_STATE.clear_playing_tracks();
    drain_plays();

    let saved = |id: &str, s: f64| SavedTrack { room_id: "r".into(), track_id: id.into(), seconds: s };
    save(
        &dir,
        &ShowState {
            saved_at_ms: now_ms() - 2_000,
            active_room_id: Some("r".into()),
            // loopA: 반복이라 언제나 이어 감 / sfxShort: 15초 중 8초 → 남은 7초 → 버림 / sfxLong: 30초 중 10초 → 남은 20초 → 이어 감
            tracks: vec![saved("loopA", 2.0), saved("sfxShort", 8.0), saved("sfxLong", 10.0)],
        },
    )
    .unwrap();
    show_state::start(dir.clone(), Duration::from_secs(3600));

    show_state::resume(now_ms()).unwrap();
    let plays = drain_plays();
    assert_eq!(plays.len(), 2, "이어 튼 트랙: {plays:?}");
    assert_eq!(plays[0].0, "loopA");
    assert!((plays[0].1 - 2.0).abs() < 1e-6, "loopA 위치 {}", plays[0].1);
    assert_eq!(plays[1].0, "sfxLong");
    assert!((plays[1].1 - 10.0).abs() < 1e-6, "sfxLong 위치 {}", plays[1].1);

    let _ = std::fs::remove_dir_all(&dir);
}
