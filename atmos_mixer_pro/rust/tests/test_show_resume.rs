//! 공연 상태 저장과 이어 가기(core::show_state). 전역 상태를 쓰므로 테스트는 하나다.
use rust_lib_atmos_mixer_pro::api::simple::api_set_active_room;
use rust_lib_atmos_mixer_pro::audio::engine::ENGINE_GENERATION;
use rust_lib_atmos_mixer_pro::audio::playback_cursor::CURSOR_TABLE;
use rust_lib_atmos_mixer_pro::audio::player::SoundData;
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::app_signals::now_ms;
use rust_lib_atmos_mixer_pro::core::restart_resume;
use rust_lib_atmos_mixer_pro::core::show_state::{self, capture, load, save, SavedTrack, ShowState};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Arc;
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

/// 재생 명령들의 (트랙, 시작 위치 초), 트랙 이름 순
fn plays(cmds: &[AudioCommand]) -> Vec<(String, f64)> {
    let mut v: Vec<(String, f64)> = cmds
        .iter()
        .filter_map(|c| match c {
            AudioCommand::PlayTrack { instance, .. } => {
                Some((instance.track_id_str.clone(), instance.position_seconds()))
            }
            _ => None,
        })
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

fn saved(room: &str, track: &str, seconds: f64) -> SavedTrack {
    SavedTrack { room_id: room.into(), track_id: track.into(), seconds }
}

#[test]
fn 공연_상태를_저장하고_시작할_때_읽어_둔_위치부터_이어_간다() {
    let dir = std::env::temp_dir().join(format!("atmos_show_resume_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for name in ["la", "lb"] {
        tone_wav(&dir.join(format!("{name}.wav")), 5.0);
    }
    let p = |n: &str| dir.join(format!("{n}.wav")).to_string_lossy().into_owned();
    GLOBAL_STATE.sound_cache.write().unwrap().insert(
        "/oa.wav".into(),
        Arc::new(SoundData { samples: vec![0.1; 96_000], channels: 1, sample_rate: 48_000 }),
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
    drain();

    // 1) 저장 내용: 위치를 아는 인스턴스만, 방 ID와 함께. 루프·단발 모두.
    GLOBAL_STATE.add_playing_track(9_001, "la".into());
    GLOBAL_STATE.add_playing_track(9_002, "oa".into());
    GLOBAL_STATE.add_playing_track(9_003, "lb".into()); // 위치가 아직 없다(막 큐에 들어감)
    CURSOR_TABLE.clear_all();
    CURSOR_TABLE.publish(0, 9_001, 2.5);
    CURSOR_TABLE.publish(1, 9_002, 1.25);
    api_set_active_room(Some("a".into())).unwrap();
    assert_eq!(
        capture(1_000),
        ShowState {
            saved_at_ms: 1_000,
            active_room_id: Some("a".into()),
            tracks: vec![saved("a", "la", 2.5), saved("a", "oa", 1.25)],
        }
    );

    // 엔진 재시작 중(재생 목록이 비고 재개를 기다림)에도 그 트랙을 저장한다
    restart_resume::take_snapshot(ENGINE_GENERATION.load(Ordering::SeqCst));
    assert!(playing().is_empty());
    assert_eq!(capture(2_000).tracks, vec![saved("a", "la", 2.5), saved("a", "oa", 1.25)]);
    restart_resume::cancel_all();
    CURSOR_TABLE.clear_all();
    assert!(capture(3_000).tracks.is_empty());

    // 2) 시작할 때 지난 상태를 읽어 둔다. 저장 스레드가 곧 파일을 지금(빈) 상태로 덮어써도
    //    이어 가기는 읽어 둔 상태를 쓴다.
    let previous = ShowState {
        saved_at_ms: now_ms() - 3_000,
        active_room_id: Some("b".into()),
        tracks: vec![saved("b", "lb", 2.5), saved("a", "oa", 1.5)],
    };
    save(&dir, &previous).unwrap();
    show_state::start(dir.clone(), Duration::from_millis(100));
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(
        load(&dir).unwrap().map(|s| s.tracks),
        Some(vec![]),
        "저장 스레드가 파일을 지금 상태로 덮어쓰지 않았다"
    );

    drain();
    show_state::resume(now_ms()).unwrap();
    assert_eq!(*GLOBAL_STATE.active_room_id.read().unwrap(), Some("b".to_string()));
    let p = plays(&drain());
    assert_eq!(p.len(), 2, "이어 튼 트랙: {p:?}");
    assert_eq!(p[0].0, "lb");
    assert!((p[0].1 - 2.5).abs() < 1e-6, "lb 시작 위치 {}", p[0].1);
    assert_eq!(p[1].0, "oa");
    assert!((p[1].1 - 1.5).abs() < 1e-6, "oa 시작 위치 {}", p[1].1);
    assert_eq!(playing(), vec!["lb".to_string(), "oa".to_string()]);

    // 3) 읽어 둔 상태는 한 번만 쓴다. 다시 부르면 첫 방 테마로 시작한다.
    show_state::resume(now_ms()).unwrap();
    assert_eq!(*GLOBAL_STATE.active_room_id.read().unwrap(), Some("a".to_string()));
    assert_eq!(playing(), vec!["la".to_string()]);

    drain();
    let _ = std::fs::remove_dir_all(&dir);
}
