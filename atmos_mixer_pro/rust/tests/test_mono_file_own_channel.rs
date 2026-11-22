//! 모노 파일은 자기 채널(CHk) 하나로만 나간다(2026-11-22 사용자 "8,9번은 지금 진행").
//!
//! 화면은 모노 파일에 모노 자리만 내놓고(2026-11-21 "모노 파일은 모노 자리만"), 엔진도 모노 줄 k로 판단한다
//! (`core::output_groups::required_group`). 그런데 새 트랙 기본값(`output_stereo: true`)이 남은 모노 파일은 믹서가 옆
//! 채널 CHk+1로도 올려 냈다(예전 "모노 소스 -> 스테레오 쌍 업믹스"). 트랙 카드가 화면에 뜨면 표시를 모노로 고쳤지만,
//! 카드를 그리기 전(무인 시작 등)에는 CHk+1이 다른 줄로 열려 있으면 거기로도 소리가 났다. 이제 업믹스하지 않는다.
//!
//! 예전 업믹스 경로의 회귀(닫힌 채널로 새기, 오른쪽에만 궤적 게인)도 함께 지킨다: 옆 채널이 열려 있고 궤적 게인이
//! 1.0이 아니어도 옆 채널은 조용해야 한다.

use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// 모노 파일을 CH1(스테레오 출력 표시가 남은 채로)로 보내는 믹서를 만든다.
fn run_mono_to_stereo(
    out_channels: usize,
    enable_left: bool,
    enable_right: bool,
    right_spatial_gain: f32,
) -> Vec<f32> {
    let (gc_tx, _) = crossbeam_channel::unbounded();
    let mut mixer = AudioMixer::new(48000, out_channels, 512, gc_tx, None);

    for ch in 0..GLOBAL_STATE.enabled_channels.len() {
        GLOBAL_STATE.enabled_channels[ch].store(false, Ordering::SeqCst);
    }
    GLOBAL_STATE.enabled_channels[0].store(enable_left, Ordering::SeqCst);
    GLOBAL_STATE.enabled_channels[1].store(enable_right, Ordering::SeqCst);
    // 모노 파일의 자리는 모노 줄 CH1이다(core::output_groups). 왼쪽 채널은 그 줄이 켜져야 열리므로 줄 표도 같이 맞춘다
    // (오른쪽 CH2는 다른 줄 — 예: 모노 CH2 — 이 연다).
    GLOBAL_STATE.mono_rows_open[0].store(enable_left, Ordering::SeqCst);

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
        0,    // output_channel: 0-based
        true, // output_stereo: 새 트랙 기본값이 남은 모노 파일
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
fn mono_file_plays_only_on_its_own_channel() {
    // --- 시나리오 1: CH1·CH2 모두 열림 -> CH1에만 소리 ---
    // 옆 채널 CH2가 다른 줄로 열려 있고 궤적 게인을 0.1로 심어도 CH2는 조용해야 한다. 예전에는 CH2로도 올려 냈다.
    let out = run_mono_to_stereo(2, true, true, 0.1);
    let left: f32 = out.iter().step_by(2).map(|s| s.abs()).sum();
    let right: f32 = out.iter().skip(1).step_by(2).map(|s| s.abs()).sum();

    assert!(left > 0.0, "자기 채널 CH1에는 소리가 있어야 한다");
    assert_eq!(right, 0.0, "옆 채널 CH2로는 소리가 나가면 안 된다. right={}", right);

    // --- 시나리오 2: 오른쪽 닫힘 -> CH1에만 소리 ---
    let out = run_mono_to_stereo(2, true, false, 1.0);
    let left: f32 = out.iter().step_by(2).map(|s| s.abs()).sum();
    let right: f32 = out.iter().skip(1).step_by(2).map(|s| s.abs()).sum();

    assert!(left > 0.0, "열린 왼쪽 채널에는 소리가 있어야 한다");
    assert_eq!(
        right, 0.0,
        "닫힌 오른쪽 채널로는 소리가 나가면 안 된다. right={}",
        right
    );

    // --- 시나리오 3: 왼쪽(트랙 자리 모노 CH1 줄) 꺼짐 -> 무음 ---
    // 트랙 출력 자리(모노 파일은 모노 줄 CH1)가 꺼져 있으면 화면에 "출력 설정에 없음"이 뜨고 엔진도 그 트랙을
    // 무음으로 둔다(사용자 결정 2026-11-21 "A. 안내 뜨는 트랙은 무음", 2026-11-22 "이전처럼 에이블톤처럼 따로",
    // core::output_groups). 예전에는 열린 오른쪽으로만 소리가 났다.
    let out = run_mono_to_stereo(2, false, true, 1.0);
    let left: f32 = out.iter().step_by(2).map(|s| s.abs()).sum();
    let right: f32 = out.iter().skip(1).step_by(2).map(|s| s.abs()).sum();

    assert_eq!(left, 0.0, "닫힌 왼쪽 채널로는 소리가 나가면 안 된다");
    assert_eq!(right, 0.0, "시작 채널이 닫힌 트랙은 열린 오른쪽으로도 소리가 나면 안 된다");
}
