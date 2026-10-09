//! 믹서 처리 폭(core::processing_channels). 장치는 최대 채널로 열지만 믹서는 출력 설정에서 켠
//! 가장 높은 채널까지만 처리한다. 2026-10-08 Windows RME MADIface USB(94채널)에서 멀티 그룹 때문에
//! 94채널 전체를 처리해 콜백이 예산(48kHz·1024프레임 21.3ms)을 넘었고 소리가 끊기고 느려졌다.
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, ChannelSetting};
use rust_lib_atmos_mixer_pro::core::processing_channels::{mixer_width, processing_channel_count};

fn setting(enabled: bool) -> ChannelSetting {
    ChannelSetting {
        enabled,
        custom_name: String::new(),
        delay_ms: 0.0,
        eq_bands: Vec::new(),
        position: None,
        phase_invert: false,
        gain_db: 0.0,
    }
}

fn on() -> ChannelSetting {
    setting(true)
}

fn off() -> ChannelSetting {
    setting(false)
}

#[test]
fn 출력_설정을_건드리지_않았으면_장치_전체를_처리한다() {
    let config = AppConfig::default();
    assert_eq!(processing_channel_count(&config, 94), 94);
    let mut all_off = AppConfig::default();
    all_off.mono_configs.insert(1, off());
    all_off.multi_configs.insert(1, off());
    assert_eq!(processing_channel_count(&all_off, 94), 94, "켠 그룹이 없으면 전체 개방과 같다");
}

#[test]
fn 채널을_켜는_화면처럼_모노_스테레오_멀티가_같이_켜져_있으면_가장_높은_채널까지_처리한다() {
    // 이 PC의 실제 설정 모양: 모노 1~40, 스테레오 홀수 1~39, 멀티 1~40, 쓰지 않는 255번.
    let mut config = AppConfig::default();
    for k in 1..=40 {
        config.mono_configs.insert(k, on());
        config.multi_configs.insert(k, on());
    }
    for k in (1..=39).step_by(2) {
        config.stereo_configs.insert(k, on());
    }
    config.mono_configs.insert(255, on());
    config.stereo_configs.insert(255, on());
    assert_eq!(processing_channel_count(&config, 94), 40);
}

#[test]
fn 스테레오는_짝_채널까지_센다() {
    let mut config = AppConfig::default();
    config.stereo_configs.insert(39, on());
    assert_eq!(processing_channel_count(&config, 94), 40);
    // 장치 마지막 채널에서 시작한 스테레오는 장치 끝을 넘지 않는다.
    let mut edge = AppConfig::default();
    edge.stereo_configs.insert(94, on());
    assert_eq!(processing_channel_count(&edge, 94), 94);
}

#[test]
fn 멀티가_가장_높은_채널이면_길이를_알_수_없어_장치_끝까지_처리한다() {
    let mut config = AppConfig::default();
    for k in 1..=8 {
        config.mono_configs.insert(k, on());
    }
    config.multi_configs.insert(1, on());
    assert_eq!(processing_channel_count(&config, 94), 8, "멀티 시작이 모노 범위 안이면 모노 끝까지");
    config.multi_configs.insert(9, on());
    assert_eq!(processing_channel_count(&config, 94), 94, "멀티 시작이 맨 위면 파일 채널 수를 몰라 장치 끝까지");
}

#[test]
fn 장치_채널_수를_넘는_번호는_세지_않는다() {
    let mut config = AppConfig::default();
    config.mono_configs.insert(2, on());
    config.mono_configs.insert(120, on());
    assert_eq!(processing_channel_count(&config, 94), 2);
    let mut only_out_of_range = AppConfig::default();
    only_out_of_range.mono_configs.insert(255, on());
    assert_eq!(processing_channel_count(&only_out_of_range, 94), 94, "범위 안에 켠 채널이 없으면 지금처럼 전체");
}

#[test]
fn 믹서_폭은_최소_16이고_장치보다_넓을_수_있다() {
    // 2채널 장치도 믹서는 16채널 확장 버스로 돈다(엔진의 기존 동작).
    assert_eq!(mixer_width(2), 16);
    assert_eq!(mixer_width(8), 16);
    assert_eq!(mixer_width(40), 40);
    assert_eq!(mixer_width(94), 94);
}
