//! 미리 읽기에 실패한 메모리 트랙(검토 A5)과 빠진 오디오 형식(T1, AAC·M4A).
//!
//! 예전에는 읽지 못한 파일을 빈 데이터로 캐시에 넣어(디스크를 계속 두드리지 않으려고) 그 트랙을 재생하면
//! Ok가 돌아오고 소리가 없었다. OSC도 `started`를 보냈다. 이제 빈 캐시 항목은 "없음"으로 보고 다시 읽어 보며,
//! 그래도 못 읽으면 Err를 돌려준다(OSC는 `rejected play_failed`). 파일을 고치면 다음 재생은 된다.
//! 전역 상태를 쓰므로 이 파일의 시험은 하나씩 돈다.
use rust_lib_atmos_mixer_pro::api::simple::{api_play_track, api_preload_all_sounds};
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());

fn tone_wav(path: &std::path::Path, seconds: f32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 48_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).expect("시험 파일");
    for i in 0..(48_000.0 * seconds) as usize {
        let v = (i as f32 * 200.0 * std::f32::consts::TAU / 48_000.0).sin() * 3000.0;
        w.write_sample(v as i16).expect("샘플 쓰기");
    }
    w.finalize().expect("시험 파일 닫기");
}

fn config_with(path: &str) -> AppConfig {
    let mut config = AppConfig::default();
    config.rooms.push(RoomConfig {
        id: "room_a5".into(),
        name: "테마 1".into(),
        color_hex: "#ffffff".into(),
        volume: 1.0,
        clear_osc_address: String::new(),
        tracks: vec![TrackConfig {
            id: "sfx_a5".into(),
            name: "효과음".into(),
            file_path: path.to_string(),
            volume: 1.0,
            is_loop: false,
            is_streaming: false,
            output_channel: 0,
            output_stereo: false,
            play_osc_address: String::new(),
            stop_osc_address: String::new(),
            routes: vec![],
        }],
    });
    config
}

fn is_playing(track_id: &str) -> bool {
    GLOBAL_STATE.playing_track_ids.read().expect("재생 목록").values().any(|id| id == track_id)
}

#[test]
fn 읽지_못한_파일은_재생을_실패로_알리고_고치면_다시_재생된다() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("atmos_preload_fail_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("시험 폴더");
    let file = dir.join("broken.wav");
    std::fs::write(&file, b"this is not audio data at all").expect("깨진 파일");
    let path = file.to_string_lossy().into_owned();

    GLOBAL_STATE.engine_sample_rate.store(48_000, Ordering::SeqCst);
    GLOBAL_STATE.clear_playing_tracks();
    GLOBAL_STATE.sound_cache.write().expect("캐시").remove(&path);
    GLOBAL_STATE.preloaded_sounds.write().expect("미리 읽은 음원").remove(&path);

    // 미리 읽기는 실패를 알리고(Err 아님 — 다른 트랙은 계속 올린다) 빈 항목을 캐시에 둔다.
    let _ = api_preload_all_sounds(config_with(&path));
    let cached = GLOBAL_STATE.sound_cache.read().expect("캐시").get(&path).cloned();
    assert!(cached.is_some_and(|d| d.samples.is_empty()), "읽지 못한 파일은 빈 항목으로 남는다(디스크 반복 방지)");

    let result = api_play_track("room_a5".into(), "sfx_a5".into());
    let err = result.expect_err("읽지 못한 파일의 재생이 성공으로 돌아왔다(소리 없는 성공)");
    assert!(err.message.contains("broken.wav"), "어느 파일인지 알린다: {}", err.message);
    assert!(!is_playing("sfx_a5"), "실패한 트랙이 재생 목록에 남았다");

    // 파일을 고치면 다음 재생은 다시 읽어 성공하고 캐시도 채운다.
    tone_wav(&file, 0.2);
    api_play_track("room_a5".into(), "sfx_a5".into()).expect("고친 파일은 재생된다");
    assert!(is_playing("sfx_a5"));
    let cached = GLOBAL_STATE.sound_cache.read().expect("캐시").get(&path).cloned();
    assert!(cached.is_some_and(|d| !d.samples.is_empty()), "고친 파일을 캐시에 넣는다");

    GLOBAL_STATE.clear_playing_tracks();
    let _ = std::fs::remove_dir_all(&dir);
}

/// T1(사용자 결정 2026-11-16): AAC·M4A만 뺀다. 나머지 형식은 그대로 읽는다.
#[test]
fn aac_디코더와_mp4_컨테이너는_빠지고_나머지_형식은_남는다() {
    use symphonia::core::codecs::*;
    let codecs = symphonia::default::get_codecs();
    assert!(codecs.get_codec(CODEC_TYPE_AAC).is_none(), "AAC 디코더가 남아 있다");
    for (name, ty) in [
        ("PCM", CODEC_TYPE_PCM_S16LE),
        ("FLAC", CODEC_TYPE_FLAC),
        ("MP3", CODEC_TYPE_MP3),
        ("MP2", CODEC_TYPE_MP2),
        ("MP1", CODEC_TYPE_MP1),
        ("Vorbis", CODEC_TYPE_VORBIS),
        ("ALAC", CODEC_TYPE_ALAC),
        ("ADPCM", CODEC_TYPE_ADPCM_IMA_WAV),
    ] {
        assert!(codecs.get_codec(ty).is_some(), "{name} 디코더가 빠졌다");
    }
}

#[test]
fn m4a_파일은_읽지_못하고_오류를_돌려준다() {
    let dir = std::env::temp_dir().join(format!("atmos_m4a_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("시험 폴더");
    let file = dir.join("song.m4a");
    // MP4 컨테이너 첫 상자(ftyp, 브랜드 M4A). isomp4가 없으면 형식을 알아보지 못한다.
    let mut bytes = vec![0, 0, 0, 0x20];
    bytes.extend_from_slice(b"ftypM4A \0\0\0\0M4A mp42isom\0\0\0\0");
    bytes.extend_from_slice(&[0u8; 64]);
    std::fs::write(&file, &bytes).expect("시험 파일");
    let r = rust_lib_atmos_mixer_pro::audio::player::SoundData::load_from_file_with_source(&file, 48_000);
    assert!(r.is_err(), "M4A를 읽었다");
    let _ = std::fs::remove_dir_all(&dir);
}
