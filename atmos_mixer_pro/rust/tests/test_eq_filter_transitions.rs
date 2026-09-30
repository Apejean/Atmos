//! EQ 밴드 전환 결함 두 가지(실측으로 확정):
//! 1. freq/gain/Q가 그대로이고 **타입만** 바뀌면 계수를 다시 계산하지 않아
//!    옛 타입으로 계속 걸렸다(Bell -> HighShelf 변경 후 12kHz 실측 0.01dB,
//!    정상 +6.02dB).
//! 2. 밴드를 끄거나 켜는 순간 필터를 즉시 우회/투입해서 파형이 튀었다
//!    (+12dB 밴드를 끌 때 샘플간 변화 5.5배). 자동 EQ는 드래그 중에 밴드를
//!    켰다 껐다 하므로 틱 소리가 된다.

use rust_lib_atmos_mixer_pro::audio::dsp::dsp_utils::ChannelDspState;
use rust_lib_atmos_mixer_pro::common::config::{EqBand, EqType};

const FS: f32 = 48_000.0;

fn band(t: EqType, enabled: bool) -> EqBand {
    EqBand { enabled, freq: 1000.0, gain: 6.0, q_factor: 2.0, filter_type: t, slope_db_per_oct: 12 }
}

fn settle(d: &mut ChannelDspState) { for _ in 0..24_000 { let _ = d.process(0.0, FS); } }

fn level_db(d: &mut ChannelDspState, f: f32) -> f32 {
    let mut ph = 0.0f32;
    for _ in 0..9_600 { let _ = d.process(ph.sin() * 0.25, FS); ph += 2.0 * std::f32::consts::PI * f / FS; }
    let (mut si, mut so) = (0.0f64, 0.0f64);
    for _ in 0..9_600 {
        let x = ph.sin() * 0.25; ph += 2.0 * std::f32::consts::PI * f / FS;
        let y = d.process(x, FS);
        si += (x as f64).powi(2); so += (y as f64).powi(2);
    }
    10.0 * (so / si).log10() as f32
}

#[test]
fn changing_only_the_type_takes_effect() {
    let mut base = ChannelDspState::new(); settle(&mut base);
    let b0 = level_db(&mut base, 12_000.0);
    let mut d = ChannelDspState::new();
    d.update_eq_targets(&[band(EqType::Bell, true)], FS); settle(&mut d);
    d.update_eq_targets(&[band(EqType::HighShelf, true)], FS); settle(&mut d);
    let got = level_db(&mut d, 12_000.0) - b0;
    assert!((got - 6.0).abs() < 1.0,
        "Bell -> HighShelf로 타입만 바꿨는데 12kHz가 {got:.2}dB다(기대 약 +6dB)");
}

/// 440Hz 사인에 +12dB 벨을 걸어 두고, 도중에 켜짐 상태를 바꿨을 때
/// 전후 샘플간 최대 변화 비율.
fn toggle_jump_ratio(start_enabled: bool) -> f32 {
    let loud = |en| EqBand { enabled: en, freq: 440.0, gain: 12.0, q_factor: 1.0, filter_type: EqType::Bell, slope_db_per_oct: 12 };
    let mut d = ChannelDspState::new();
    d.update_eq_targets(&[loud(start_enabled)], FS);
    let mut ph = 0.0f32;
    let mut prev = 0.0f32;
    let (mut before, mut after) = (0.0f32, 0.0f32);
    for n in 0..48_000 {
        if n == 24_000 { d.update_eq_targets(&[loud(!start_enabled)], FS); }
        let y = d.process(ph.sin() * 0.1, FS);
        ph += 2.0 * std::f32::consts::PI * 440.0 / FS;
        let j = (y - prev).abs();
        prev = y;
        if (12_000..24_000).contains(&n) { before = before.max(j); }
        if (24_000..36_000).contains(&n) { after = after.max(j); }
    }
    // 켜진 쪽(큰 레벨)을 기준으로 삼아, 전환 순간의 계단만 잡는다.
    after / before.max(after.min(before))
}

#[test]
fn disabling_a_band_does_not_step_the_waveform() {
    let mut d = ChannelDspState::new();
    let loud = |en| EqBand { enabled: en, freq: 440.0, gain: 12.0, q_factor: 1.0, filter_type: EqType::Bell, slope_db_per_oct: 12 };
    d.update_eq_targets(&[loud(true)], FS);
    let mut ph = 0.0f32;
    let mut prev = 0.0f32;
    let (mut steady_on, mut transition) = (0.0f32, 0.0f32);
    for n in 0..48_000 {
        if n == 24_000 { d.update_eq_targets(&[loud(false)], FS); }
        let y = d.process(ph.sin() * 0.1, FS);
        ph += 2.0 * std::f32::consts::PI * 440.0 / FS;
        let j = (y - prev).abs();
        prev = y;
        if (12_000..24_000).contains(&n) { steady_on = steady_on.max(j); }
        if (24_000..24_600).contains(&n) { transition = transition.max(j); }
    }
    // 켜진 상태(+12dB)의 기울기보다 커지면 계단이 생긴 것이다.
    assert!(transition <= steady_on * 1.05,
        "밴드를 끄는 순간 파형이 튄다: 켜짐 {steady_on:.5} / 전환 {transition:.5}");
    let _ = toggle_jump_ratio;
}

#[test]
fn enabling_a_band_does_not_step_the_waveform() {
    let loud = |en| EqBand { enabled: en, freq: 440.0, gain: 12.0, q_factor: 1.0, filter_type: EqType::Bell, slope_db_per_oct: 12 };
    let mut d = ChannelDspState::new();
    d.update_eq_targets(&[loud(false)], FS);
    let mut ph = 0.0f32;
    let mut prev = 0.0f32;
    let (mut transition, mut steady_on) = (0.0f32, 0.0f32);
    for n in 0..72_000 {
        if n == 24_000 { d.update_eq_targets(&[loud(true)], FS); }
        let y = d.process(ph.sin() * 0.1, FS);
        ph += 2.0 * std::f32::consts::PI * 440.0 / FS;
        let j = (y - prev).abs();
        prev = y;
        if (24_000..24_600).contains(&n) { transition = transition.max(j); }
        if (60_000..72_000).contains(&n) { steady_on = steady_on.max(j); }
    }
    assert!(transition <= steady_on * 1.05,
        "밴드를 켜는 순간 파형이 튄다: 전환 {transition:.5} / 켜진 뒤 {steady_on:.5}");
}
