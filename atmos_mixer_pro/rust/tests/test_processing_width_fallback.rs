//! 출력 설정(모노·스테레오·멀티 그룹)이 비었을 때 믹서 처리 폭(설계 docs/superpowers/specs/2026-11-20-processing-width-fallback-design.md).
//! 예전에는 장치 전체(94채널)를 처리해 1.3.13 확인(2026-11-20)에서 처리 부하가 예산(21.3ms)을 넘었다. 이제 트랙이 쓰는
//! 가장 높은 채널과 서브우퍼 채널 중 큰 값까지만 처리한다. 켠 그룹이 있으면 지금 규칙 그대로다.
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, ChannelSetting, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::processing_channels::{effective_processing_channels, used_channel_ceiling};

fn track(path: &str, output_channel: u32, output_stereo: bool) -> TrackConfig {
    TrackConfig {
        id: path.into(),
        name: path.into(),
        file_path: path.into(),
        volume: 1.0,
        is_loop: false,
        is_streaming: true,
        output_channel,
        output_stereo,
        play_osc_address: String::new(),
        stop_osc_address: String::new(),
    }
}

fn config(tracks: Vec<TrackConfig>) -> AppConfig {
    AppConfig {
        rooms: vec![RoomConfig {
            id: "r".into(),
            name: "r".into(),
            color_hex: "#ffffff".into(),
            volume: 1.0,
            clear_osc_address: String::new(),
            tracks,
        }],
        ..AppConfig::default()
    }
}

/// 시험용 파일 채널 수: 이름 앞부분으로 정한다. "x"로 시작하면 읽지 못하는 파일.
fn channels(path: &str) -> Option<usize> {
    match path.split('_').next()? {
        "mono" => Some(1),
        "stereo" => Some(2),
        "ch24" => Some(24),
        "ch6" => Some(6),
        _ => None,
    }
}

// 트랙의 `output_channel`은 0부터 센 장치 채널 번호다(CH1 = 0, Dart `TrackConfig.outputChannel`·믹서 audio/mixer.rs와 같다).
// 1.3.14는 1부터 센다고 보고 계산해 맨 위 채널 하나를 빠뜨렸다(2026-11-21 설치본 확인: CH25·26 스테레오 트랙인데 25채널만 처리).
#[test]
fn 트랙이_쓰는_가장_높은_채널까지_센다() {
    assert_eq!(used_channel_ceiling(&config(vec![track("mono_a", 4, false)]), 94, &channels, 0), 5, "모노 트랙 CH5(0번부터 4)");
    assert_eq!(
        used_channel_ceiling(&config(vec![track("mono_a", 4, true)]), 94, &channels, 0),
        6,
        "모노 파일을 스테레오로 내면 CH5·6"
    );
    assert_eq!(
        used_channel_ceiling(&config(vec![track("ch24_long", 0, true)]), 94, &channels, 0),
        24,
        "24채널 파일 N:N은 CH1~24"
    );
    assert_eq!(
        used_channel_ceiling(&config(vec![track("ch6_a", 6, false)]), 94, &channels, 0),
        7,
        "다채널 파일도 스테레오 출력을 끄면 모노로 합쳐 CH7 하나"
    );
    let several = config(vec![track("stereo_a", 2, true), track("mono_b", 10, false), track("ch6_c", 1, true)]);
    assert_eq!(used_channel_ceiling(&several, 94, &channels, 0), 11, "가장 높은 것(CH11)");
}

#[test]
fn 맨_위_채널을_빠뜨리지_않는다() {
    // 1.3.14 설치본 확인에서 본 공연: 24채널 루프 CH1~24, 스테레오 트랙 CH25·26 → 26채널을 처리해야 한다.
    let seen = config(vec![track("ch24_long", 0, true), track("stereo_short", 24, true)]);
    assert_eq!(used_channel_ceiling(&seen, 94, &channels, 0), 26, "스테레오 CH25·26의 CH26까지");
    assert_eq!(
        used_channel_ceiling(&config(vec![track("mono_a", 16, false)]), 94, &channels, 0),
        17,
        "모노 CH17(믹서 최소 16채널 위)"
    );
}

#[test]
fn 읽지_못하는_파일은_빼고_서브우퍼_채널도_센다() {
    let with_missing = config(vec![track("x_missing", 39, true), track("stereo_a", 0, true)]);
    assert_eq!(used_channel_ceiling(&with_missing, 94, &channels, 0), 2, "읽지 못하는 파일은 재생되지 않으므로 뺀다");
    assert_eq!(
        used_channel_ceiling(&config(vec![track("stereo_a", 0, true)]), 94, &channels, 40),
        40,
        "서브우퍼 CH40은 트랙이 없어도 저역이 나간다"
    );
}

#[test]
fn 장치_채널로_자르고_아무것도_없으면_2채널이다() {
    assert_eq!(used_channel_ceiling(&config(vec![track("ch24_long", 79, true)]), 94, &channels, 0), 94, "장치 끝에서 자름");
    assert_eq!(used_channel_ceiling(&AppConfig::default(), 94, &channels, 0), 2, "트랙·서브 없음");
    assert_eq!(used_channel_ceiling(&config(vec![track("mono_a", 0, false)]), 94, &channels, 0), 2, "CH1 모노 하나도 최소 2");
}

#[test]
fn 켠_그룹이_없을_때만_트랙_기준을_쓴다() {
    let empty = config(vec![track("ch24_long", 0, true)]);
    assert_eq!(effective_processing_channels(&empty, 94, 24), 24, "출력 설정이 비었으면 트랙 기준");
    let mut configured = empty.clone();
    configured.mono_configs.insert(
        12,
        ChannelSetting {
            enabled: true,
            custom_name: String::new(),
            delay_ms: 0.0,
            eq_bands: Vec::new(),
            position: None,
            phase_invert: false,
            gain_db: 0.0,
        },
    );
    assert_eq!(effective_processing_channels(&configured, 94, 24), 12, "켠 그룹이 있으면 지금 규칙(CH12)");
}

fn wav(path: &std::path::Path, channels: u16) {
    let spec = hound::WavSpec { channels, sample_rate: 48_000, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for _ in 0..(4_800 * channels as usize) {
        w.write_sample(0i16).unwrap();
    }
    w.finalize().unwrap();
}

#[test]
fn 실제_파일_머리말과_서브우퍼_채널로_처리_폭을_정한다() {
    use rust_lib_atmos_mixer_pro::core::processing_channels::current_processing_channels;
    use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
    use std::sync::atomic::Ordering;
    let dir = std::env::temp_dir().join(format!("width_fallback_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let six = dir.join("six.wav");
    wav(&six, 6);
    let cfg = config(vec![
        track(&six.to_string_lossy(), 2, true),                                // CH3~8(0번부터 2)
        track(&dir.join("없는.wav").to_string_lossy(), 59, true),             // 읽지 못함 → 뺌
    ]);
    GLOBAL_STATE.highest_sub_channel.store(0, Ordering::SeqCst);
    assert_eq!(current_processing_channels(&cfg, 94), 8, "6채널 파일 N:N CH3 → CH8");
    GLOBAL_STATE.highest_sub_channel.store(20, Ordering::SeqCst);
    assert_eq!(current_processing_channels(&cfg, 94), 20, "서브우퍼 CH20");
    GLOBAL_STATE.highest_sub_channel.store(0, Ordering::SeqCst);
    let _ = std::fs::remove_dir_all(dir);
}
