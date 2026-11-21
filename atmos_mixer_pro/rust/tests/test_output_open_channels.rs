//! 출력 설정 = 열린 채널 하나, 트랙 출력 자리가 열려 있을 때만 소리(사용자 결정 2026-11-21 "A. 안내 뜨는 트랙은
//! 무음"·"채널 하나로 묶기", 설계 docs/superpowers/specs/2026-11-21-output-config-open-channels-design.md).
//! 예전에는 엔진이 채널 게이트만 봐서, 화면에 "Output Config에서 사용할 채널 그룹을 열어주세요"가 뜬 트랙도 열린
//! 채널로 소리가 났다(2026-11-21 15:21 장치 확인).
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, ChannelSetting};
use rust_lib_atmos_mixer_pro::core::output_groups::{open_channels, required_group, route_open_in, GroupKind};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Arc;

const FS: u32 = 48_000;
const BLOCK: usize = 480;
const OUT: usize = 8;

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

#[test]
fn 켠_모노_스테레오_멀티를_합쳐_열린_채널을_만든다() {
    let mut config = AppConfig::default();
    config.mono_configs.insert(3, setting(true)); // CH3
    config.stereo_configs.insert(5, setting(true)); // CH5·CH6
    config.multi_configs.insert(1, setting(true)); // CH1 하나(예전에는 끝까지)
    config.multi_configs.insert(8, setting(false)); // 꺼짐
    config.mono_configs.insert(0, setting(true)); // 0번 키는 쓰지 않는다
    config.mono_configs.insert(255, setting(true)); // 장치 밖
    assert_eq!(
        open_channels(&config, 8),
        vec![true, false, true, false, true, true, false, false]
    );
    assert_eq!(open_channels(&AppConfig::default(), 4), vec![false; 4], "켠 것이 없으면 모두 닫힘");
}

#[test]
fn 파일_채널_수와_스테레오_출력으로_자리_종류를_정한다() {
    // 화면 buildChannelRoutingItems와 같은 규칙: 다채널 파일은 멀티 자리, 모노 파일은 모노 자리(새 트랙 기본값
    // "스테레오 출력 켬"이어도 같은 채널의 모노), 스테레오 파일은 스테레오 출력이면 스테레오·아니면 모노 자리.
    assert_eq!(required_group(24, true), GroupKind::Multi);
    assert_eq!(required_group(4, false), GroupKind::Multi);
    assert_eq!(required_group(1, false), GroupKind::Mono);
    assert_eq!(required_group(1, true), GroupKind::Mono);
    assert_eq!(required_group(2, true), GroupKind::Stereo);
    assert_eq!(required_group(2, false), GroupKind::Mono);
}

#[test]
fn 자리는_시작_채널이_열려_있어야_하고_스테레오는_짝_두_채널이_열려_있어야_한다() {
    let open = |set: &'static [usize]| move |c: usize| set.contains(&c);
    assert!(route_open_in(open(&[0, 1]), GroupKind::Multi, 0), "24채널 파일도 CH1이 열려 있으면 자리가 맞다");
    assert!(!route_open_in(open(&[1, 2]), GroupKind::Multi, 0), "시작 채널 CH1이 닫힘");
    assert!(route_open_in(open(&[4]), GroupKind::Mono, 4));
    assert!(route_open_in(open(&[0, 1]), GroupKind::Stereo, 0), "스테레오 1/2");
    assert!(!route_open_in(open(&[0]), GroupKind::Stereo, 0), "CH2가 닫혀 스테레오 1/2는 아니다");
    assert!(!route_open_in(open(&[1, 2]), GroupKind::Stereo, 1), "2/3은 화면의 스테레오 짝(1/2·3/4…)이 아니다");
}

fn peak(buf: &[f32]) -> f32 {
    buf.iter().fold(0.0f32, |m, v| m.max(v.abs()))
}

/// 블록 하나를 돌리고 (전체 최대 진폭, CH1의 인접 샘플 간 최대 변화량)을 돌려준다.
fn block(mixer: &mut AudioMixer, buf: &mut [f32], prev_ch1: &mut f32) -> (f32, f32) {
    buf.fill(0.0);
    mixer.process(buf, OUT);
    let mut jump = 0.0f32;
    for frame in 0..BLOCK {
        let y = buf[frame * OUT];
        jump = jump.max((y - *prev_ch1).abs());
        *prev_ch1 = y;
    }
    (peak(buf), jump)
}

#[test]
fn 자리가_닫힌_트랙은_무음이고_위치는_흐르며_자리가_열리면_딸깍_없이_들린다() {
    // 스테레오 파일을 스테레오 1/2 자리에 둔다. CH1은 열리고 CH2는 닫혀 있다 — 채널 게이트로는 CH1이 열려 있지만
    // 스테레오 1/2 자리는 닫혀 있다(화면에 안내가 뜬다).
    for (index, slot) in GLOBAL_STATE.enabled_channels.iter().enumerate() {
        slot.store(index != 1, Ordering::SeqCst);
    }
    let (gc_tx, _gc_rx) = crossbeam_channel::bounded(16);
    let mut mixer = AudioMixer::new(FS, OUT, BLOCK, gc_tx, None);
    mixer.startup_ramp.current_gain = 1.0; // 앱 시작 뮤트 램프는 이 시험의 대상이 아니다
    // 1kHz 코사인(블록 480프레임 = 10주기, 블록 첫 샘플이 늘 마루라 한 번에 뛰면 드러난다).
    let frames = 4800;
    let mut samples = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let v = 0.5 * (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / FS as f32).cos();
        samples.push(v);
        samples.push(v);
    }
    let data = Arc::new(SoundData { samples, channels: 2, sample_rate: FS });
    mixer.instances[0] = Some(SoundInstance::new(
        1, 7, 1, "t".into(), Some(data), None, FS, 2, true, 1.0, 0, true, None,
        GLOBAL_STATE.enabled_channels.len(),
    ));
    let mut buf = vec![0.0f32; BLOCK * OUT];
    let mut prev = 0.0f32;

    // 자리가 닫힌 동안: 처음부터 무음(열린 CH1로도 새어 나오지 않는다).
    let mut loudest = 0.0f32;
    for _ in 0..200 {
        loudest = loudest.max(block(&mut mixer, &mut buf, &mut prev).0);
    }
    assert!(loudest < 1e-6, "스테레오 1/2 자리가 닫혀 있는데 소리가 났다: {loudest}");

    // 무음이어도 재생 위치는 흐른다(루프 한 바퀴가 10블록이라 7블록만 본다).
    let before = mixer.instances[0].as_ref().map(|i| i.position_seconds()).unwrap_or(0.0);
    for _ in 0..7 {
        block(&mut mixer, &mut buf, &mut prev);
    }
    let after = mixer.instances[0].as_ref().map(|i| i.position_seconds()).unwrap_or(0.0);
    assert!(after != before, "무음인 동안 재생 위치가 멈췄다");

    // CH2를 연다: 스테레오 1/2 자리가 맞아 서서히 커진다(CH1에서 한 번에 뛰지 않는다).
    GLOBAL_STATE.enabled_channels[1].store(true, Ordering::SeqCst);
    let mut worst_jump = 0.0f32;
    for _ in 0..5 {
        worst_jump = worst_jump.max(block(&mut mixer, &mut buf, &mut prev).1);
    }
    let mut steady_jump = 0.0f32;
    let mut steady_peak = 0.0f32;
    for _ in 0..50 {
        let (p, j) = block(&mut mixer, &mut buf, &mut prev);
        steady_jump = steady_jump.max(j);
        steady_peak = steady_peak.max(p);
    }
    assert!(steady_peak > 0.1, "자리가 열렸는데 소리가 나지 않는다: {steady_peak}");
    assert!(worst_jump <= steady_jump * 2.0, "자리가 열릴 때 CH1이 튀었다: 전환 {worst_jump}, 정상 {steady_jump}");

    // CH2를 다시 닫는다: CH1은 서서히 줄어 무음이 된다(CH2 자체는 채널 게이트라 바로 닫힌다 — 기존 동작).
    GLOBAL_STATE.enabled_channels[1].store(false, Ordering::SeqCst);
    let mut worst_jump = 0.0f32;
    for _ in 0..5 {
        worst_jump = worst_jump.max(block(&mut mixer, &mut buf, &mut prev).1);
    }
    assert!(worst_jump <= steady_jump * 2.0, "자리가 닫힐 때 CH1이 튀었다: {worst_jump}");
    let mut tail = 0.0f32;
    for _ in 0..20 {
        tail = tail.max(block(&mut mixer, &mut buf, &mut prev).0);
    }
    assert!(tail < 1e-6, "자리가 닫혔는데 소리가 남았다: {tail}");
}
