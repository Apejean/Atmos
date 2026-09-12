//! 모노 소스 -> 스테레오 쌍 업믹스 경로의 회귀 테스트.
//!
//! 이 경로에는 두 가지 결함이 있었다.
//! 1. `enabled_channels` 게이트를 건너뛰었다. Output Config에서 닫은 채널로도
//!    소리가 나갔고, 바로 위 N:N 라우팅은 게이트를 거치기 때문에 왼쪽은
//!    음소거되고 오른쪽만 나오는 비대칭이 생겼다.
//! 2. `channel_spatial_gains`를 오른쪽 채널에만 곱했다. 이 배열은 궤적/공간
//!    코드가 1.0에서 크게 벗어나게 변조하므로, 모노 소스를 스테레오 쌍으로
//!    보내면 왼쪽은 원래 레벨이고 오른쪽만 궤적 게인이 걸려 이미지가 한쪽으로
//!    쏠리거나 오른쪽이 사라졌다.

use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// 채널 0에서 시작하는 스테레오 쌍으로 모노 소스를 보내는 믹서를 만든다.
fn run_mono_to_stereo(
    out_channels: usize,
    enable_left: bool,
    enable_right: bool,
    right_spatial_gain: f32,
) -> Vec<f32> {
    let (gc_tx, _) = crossbeam_channel::unbounded();
    let mut mixer = AudioMixer::new(48000, out_channels, gc_tx, None);

    for ch in 0..GLOBAL_STATE.enabled_channels.len() {
        GLOBAL_STATE.enabled_channels[ch].store(false, Ordering::SeqCst);
    }
    GLOBAL_STATE.enabled_channels[0].store(enable_left, Ordering::SeqCst);
    GLOBAL_STATE.enabled_channels[1].store(enable_right, Ordering::SeqCst);

    mixer.startup_ramp.current_gain = 1.0;

    // 궤적 게인이 오른쪽에만 새는지 확인하기 위해 1.0이 아닌 값을 심는다.
    mixer.channel_spatial_gains[1] = right_spatial_gain;
    mixer.channel_spatial_gains_target[1] = right_spatial_gain;

    let sound_data = Arc::new(SoundData {
        samples: vec![1.0; 512 * 200],
        channels: 1,
        sample_rate: 48000,
    });

    let mut instance = SoundInstance::new(
        1,
        1,
        1,
        "Mono SFX".to_string(),
        Some(sound_data),
        None,
        48000,
        1,
        false,
        1.0,
        0,    // output_channel: 0-based, 쌍의 시작 채널
        true, // output_stereo
        None,
        GLOBAL_STATE.enabled_channels.len(),
    );
    instance.fade_weight = 1.0;
    mixer.instances[0] = Some(instance);

    let mut out: Vec<f32> = vec![0.0; out_channels * 512];
    for _ in 0..5 {
        mixer.process(&mut out, out_channels);
    }
    out
}

/// `GLOBAL_STATE.enabled_channels`는 프로세스 전역이므로 시나리오를 여러
/// `#[test]`로 쪼개면 병렬 실행 중 서로의 게이트 설정을 읽어 엉뚱하게 실패한다.
/// 그래서 세 시나리오를 한 테스트 안에서 순차로 돌린다
/// (`test_bed_object_matrix.rs`와 같은 방식).
#[test]
fn mono_to_stereo_upmix_respects_gate_and_stays_symmetric() {
    // --- 시나리오 1: 좌우 모두 열림 -> 레벨이 대칭이어야 한다 ---
    // 오른쪽 채널의 궤적 게인을 0.1로 심어도 결과는 대칭이어야 한다. 예전에는
    // 오른쪽에만 channel_spatial_gains가 곱해져서 레벨이 어긋났다.
    let out = run_mono_to_stereo(2, true, true, 0.1);
    let left: f32 = out.iter().step_by(2).map(|s| s.abs()).sum();
    let right: f32 = out.iter().skip(1).step_by(2).map(|s| s.abs()).sum();

    assert!(left > 0.0, "왼쪽 채널에 소리가 있어야 한다");
    assert!(right > 0.0, "오른쪽 채널에 소리가 있어야 한다");
    let ratio = right / left;
    assert!(
        (ratio - 1.0).abs() < 0.05,
        "좌우 레벨이 대칭이어야 한다. left={}, right={}, ratio={}",
        left,
        right,
        ratio
    );

    // --- 시나리오 2: 오른쪽 닫힘 -> 업믹스도 게이트를 지켜야 한다 ---
    // 예전에는 게이트를 건너뛰어 닫힌 채널로 소리가 나갔다.
    let out = run_mono_to_stereo(2, true, false, 1.0);
    let left: f32 = out.iter().step_by(2).map(|s| s.abs()).sum();
    let right: f32 = out.iter().skip(1).step_by(2).map(|s| s.abs()).sum();

    assert!(left > 0.0, "열린 왼쪽 채널에는 소리가 있어야 한다");
    assert_eq!(
        right, 0.0,
        "닫힌 오른쪽 채널로는 소리가 나가면 안 된다. right={}",
        right
    );

    // --- 시나리오 3: 왼쪽 닫힘 -> 오른쪽만 소리 ---
    // 이 경우 오른쪽만 나는 것은 게이트를 따른 정상 동작이다. 사용자가 본
    // "스테레오인데 오른쪽만 나온다"가 게이트 비대칭 결함인지 설정 때문인지
    // 구분하기 위해 남긴다.
    let out = run_mono_to_stereo(2, false, true, 1.0);
    let left: f32 = out.iter().step_by(2).map(|s| s.abs()).sum();
    let right: f32 = out.iter().skip(1).step_by(2).map(|s| s.abs()).sum();

    assert_eq!(left, 0.0, "닫힌 왼쪽 채널로는 소리가 나가면 안 된다");
    assert!(right > 0.0, "열린 오른쪽 채널에는 소리가 있어야 한다");
}
