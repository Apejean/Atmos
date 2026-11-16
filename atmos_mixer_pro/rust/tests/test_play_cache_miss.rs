//! 메모리로 읽는 트랙(반복·스트리밍 아님)이 캐시에 아직 없을 때 재생하면, 파일을 읽어 캐시에 넣고 재생한다.
//!
//! 예전에는 이 경로가 `sound_cache` 읽기 잠금을 쥔 채 같은 잠금의 쓰기를 잡아 그 스레드가 영원히 멈췄다
//! (2026-11-13 코드 검토). 설정 읽기 잠금도 쥔 채라, 그 뒤 설정 저장과 OSC 처리가 줄줄이 막혔다. 소리와
//! 하트비트는 살아 있어 감시 프로그램도 다시 띄우지 않는다. 앱을 켠 직후나 설정을 저장한 직후처럼 미리
//! 불러오기가 끝나기 전에 재생 명령(OSC 포함)이 오면 이 경로를 탄다. 전역 상태를 쓰므로 테스트는 하나다.
use rust_lib_atmos_mixer_pro::api::simple::api_play_track;
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
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
    let mut w = hound::WavWriter::create(path, spec).expect("시험 파일");
    for i in 0..(48_000.0 * seconds) as usize {
        let v = (i as f32 * 200.0 * std::f32::consts::TAU / 48_000.0).sin() * 3000.0;
        w.write_sample(v as i16).expect("샘플 쓰기");
    }
    w.finalize().expect("시험 파일 닫기");
}

#[test]
fn 캐시에_없는_메모리_트랙을_재생해도_멈추지_않고_캐시에_넣어_재생한다() {
    let dir = std::env::temp_dir().join(format!("atmos_play_cache_miss_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("시험 폴더");
    let wav = dir.join("sfx.wav");
    tone_wav(&wav, 0.2);
    let path = wav.to_string_lossy().into_owned();

    GLOBAL_STATE.engine_sample_rate.store(48_000, Ordering::SeqCst);
    let mut config = AppConfig::default();
    config.rooms.push(RoomConfig {
        id: "room_miss".into(),
        name: "테마 1".into(),
        color_hex: "#ffffff".into(),
        volume: 1.0,
        clear_osc_address: String::new(),
        tracks: vec![TrackConfig {
            id: "sfx".into(),
            name: "효과음".into(),
            file_path: path.clone(),
            volume: 1.0,
            is_loop: false,
            is_streaming: false,
            output_channel: 0,
            output_stereo: false,
            play_osc_address: String::new(),
            stop_osc_address: String::new(),
        }],
    });
    *GLOBAL_STATE.config.write().expect("설정") = Some(config);
    GLOBAL_STATE.clear_playing_tracks();
    // 미리 불러오기가 아직 끝나지 않은 상태: 두 캐시 모두 이 파일이 없다.
    GLOBAL_STATE.sound_cache.write().expect("캐시").remove(&path);
    GLOBAL_STATE.preloaded_sounds.write().expect("미리 읽은 음원").remove(&path);

    // 멈추면 테스트가 끝나지 않으므로 다른 스레드에서 부르고 기한을 둔다.
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(api_play_track("room_miss".into(), "sfx".into()));
    });
    let result = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("재생 호출이 10초 안에 돌아오지 않았다(sound_cache 잠금 교착)");
    result.expect("캐시에 없는 트랙의 재생이 실패했다");

    assert!(
        GLOBAL_STATE.sound_cache.read().expect("캐시").contains_key(&path),
        "읽은 음원이 캐시에 들어가지 않았다"
    );
    assert!(
        GLOBAL_STATE
            .playing_track_ids
            .read()
            .expect("재생 목록")
            .values()
            .any(|id| id == "sfx"),
        "재생한 트랙이 재생 목록에 없다"
    );
    // 재생 호출이 설정 잠금을 놓았다(설정 저장이 막히지 않는다).
    assert!(GLOBAL_STATE.config.try_write().is_ok(), "설정 잠금이 풀리지 않았다");

    let _ = std::fs::remove_dir_all(&dir);
}
