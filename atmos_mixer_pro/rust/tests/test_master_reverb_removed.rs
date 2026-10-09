//! 마스터 리버브는 출력에 걸지 않는다(사용자 결정 2026-10-09).
//!
//! 예전에는 ALL OUTPUTS 리버브 설정(기본 홀 80%)이 마스터 리버브(`mixer.reverb`)로 하드웨어
//! CH1·CH2에만 걸렸다. 그래서 CH1·CH2의 채널 리버브를 0%로 내려도 스피커에 홀 리버브가 크게 남았고
//! (2026-10-08 Windows 실기: "리버브 걸리는 지연된 소리"), L/R을 모노로 합쳐 넣어 CH2 소리가 CH1으로
//! 샜다(HANDOFF 남은 일 8). 리버브는 채널 DSP의 채널별 리버브만 쓴다.

use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Arc;

const FS: u32 = 48_000;
const CH: usize = 16;
const BLOCK: usize = 512;
/// 0.2초 잡음 뒤 1초 무음. 잔향이 있으면 무음 구간에 남는다.
const INPUT_SEC: f32 = 0.2;
const TAIL_SEC: f32 = 1.0;

fn noise(seed: &mut u32) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    (*seed as f32 / u32::MAX as f32) * 2.0 - 1.0
}

/// 홀 80%(예전 ALL OUTPUTS 기본값).
fn set_hall_80(reverb: &mut rust_lib_atmos_mixer_pro::audio::reverb::VirtualRoomReverb) {
    reverb.set_full_params(true, 800.0, 3.2, 25.0, 40.0, 80.0, 0.8);
}

/// 잡음 한 번을 `output_channel`(0 = CH1)에 모노로 보낸다.
fn mixer_with_burst(output_channel: usize) -> AudioMixer {
    let (gc_tx, _gc_rx) = crossbeam_channel::unbounded();
    let mut mixer = AudioMixer::new(FS, CH, BLOCK, gc_tx, None);
    for ch in 0..CH {
        GLOBAL_STATE.enabled_channels[ch].store(true, Ordering::SeqCst);
    }
    mixer.startup_ramp.current_gain = 1.0;

    let mut seed = 0x1234_5678u32;
    let n_in = (FS as f32 * INPUT_SEC) as usize;
    let n_tail = (FS as f32 * TAIL_SEC) as usize;
    let mut samples: Vec<f32> = (0..n_in).map(|_| noise(&mut seed) * 0.3).collect();
    samples.resize(n_in + n_tail, 0.0);
    let data = Arc::new(SoundData { samples, channels: 1, sample_rate: FS });
    let mut inst = SoundInstance::new(
        1,
        1,
        1,
        "burst".to_string(),
        Some(data),
        None,
        FS,
        1,
        false,
        1.0,
        output_channel,
        false,
        None,
        GLOBAL_STATE.enabled_channels.len(),
    );
    inst.fade_weight = 1.0;
    mixer.instances[0] = Some(inst);
    mixer
}

/// 전체 길이를 렌더해서 CH1·CH2 샘플을 돌려준다.
fn render(mixer: &mut AudioMixer) -> (Vec<f32>, Vec<f32>) {
    let total = (FS as f32 * (INPUT_SEC + TAIL_SEC)) as usize;
    let blocks = total.div_ceil(BLOCK);
    let mut buf = vec![0.0f32; CH * BLOCK];
    let (mut ch1, mut ch2) = (Vec::new(), Vec::new());
    for _ in 0..blocks {
        buf.fill(0.0);
        mixer.process(&mut buf, CH);
        for f in 0..BLOCK {
            ch1.push(buf[f * CH]);
            ch2.push(buf[f * CH + 1]);
        }
    }
    (ch1, ch2)
}

fn max_abs_diff(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0, f32::max)
}

/// 입력이 끝난 뒤(무음 구간)의 에너지.
fn tail_energy(y: &[f32]) -> f64 {
    let start = (FS as f32 * INPUT_SEC) as usize + BLOCK;
    y[start..].iter().map(|v| (*v as f64) * (*v as f64)).sum()
}

#[test]
fn 마스터_리버브_값을_넣어도_CH1_소리가_바뀌지_않는다() {
    let mut plain = mixer_with_burst(0);
    let (plain_ch1, _) = render(&mut plain);

    let mut with_master = mixer_with_burst(0);
    set_hall_80(&mut with_master.reverb);
    let (master_ch1, _) = render(&mut with_master);

    let diff = max_abs_diff(&plain_ch1, &master_ch1);
    assert!(diff < 1e-9, "마스터 리버브가 CH1에 걸렸다: 최대 차이 {diff:e}");
}

#[test]
fn 마스터_리버브_값이_있어도_CH2_소리가_CH1으로_새지_않는다() {
    let mut mixer = mixer_with_burst(1);
    set_hall_80(&mut mixer.reverb);
    let (ch1, ch2) = render(&mut mixer);
    let ch1_peak = ch1.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    let ch2_peak = ch2.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    assert!(ch2_peak > 0.05, "CH2에 소리가 나가지 않았다(테스트 설정 오류): {ch2_peak}");
    assert_eq!(ch1_peak, 0.0, "CH2 소리가 CH1으로 샜다: CH1 최대 {ch1_peak}");
}

#[test]
fn 채널별_리버브는_그대로_걸린다() {
    let mut dry = mixer_with_burst(0);
    let (dry_ch1, _) = render(&mut dry);

    let mut wet = mixer_with_burst(0);
    set_hall_80(&mut wet.channel_dsp[0].reverb);
    let (wet_ch1, _) = render(&mut wet);

    let dry_tail = tail_energy(&dry_ch1);
    let wet_tail = tail_energy(&wet_ch1);
    assert!(
        wet_tail > dry_tail * 100.0 && wet_tail > 1e-3,
        "CH1 채널 리버브의 잔향이 없다: 리버브 {wet_tail:.6} / 원음 {dry_tail:.6}"
    );
}
