//! 로우컷/하이컷의 슬로프(12/18/24dB/oct)가 실제 소리에 걸리는지 실측한다.
//!
//! 예전에는 슬로프가 EQ 화면 표시에만 있고, 백엔드로 가는 EqBand에 필드가 없어
//! DSP는 늘 단일 2차 필터(12dB/oct)였다. 화면은 24를 그리는데 소리는 12였다
//! (실기 보고: "정확한 Slope 값을 사용해 적용해야 한다").
//!
//! 기준은 Butterworth 응답이다. 차단 주파수에서 한 옥타브 떨어진 지점의 감쇠:
//! 2차 -12.30dB / 3차 -18.13dB / 4차 -24.10dB.

use rust_lib_atmos_mixer_pro::audio::dsp::dsp_utils::ChannelDspState;
use rust_lib_atmos_mixer_pro::common::config::{EqBand, EqType};

const FS: f32 = 48_000.0;

fn cut(t: EqType, freq: f32, slope: u32) -> EqBand {
    EqBand { enabled: true, freq, gain: 0.0, q_factor: 0.707, filter_type: t, slope_db_per_oct: slope }
}

fn settle(d: &mut ChannelDspState) { for _ in 0..24_000 { let _ = d.process(0.0, FS); } }

fn level_db(d: &mut ChannelDspState, f: f32) -> f32 {
    let mut ph = 0.0f32;
    for _ in 0..19_200 { let _ = d.process(ph.sin() * 0.25, FS); ph += 2.0 * std::f32::consts::PI * f / FS; }
    let (mut si, mut so) = (0.0f64, 0.0f64);
    for _ in 0..19_200 {
        let x = ph.sin() * 0.25; ph += 2.0 * std::f32::consts::PI * f / FS;
        let y = d.process(x, FS);
        si += (x as f64).powi(2); so += (y as f64).powi(2);
    }
    10.0 * (so / si).log10() as f32
}

fn response(band: EqBand, probe: f32) -> f32 {
    let mut base = ChannelDspState::new(); settle(&mut base);
    let b0 = level_db(&mut base, probe);
    let mut d = ChannelDspState::new(); d.update_eq_targets(&[band], FS); settle(&mut d);
    level_db(&mut d, probe) - b0
}

#[test]
fn low_cut_slope_reaches_the_dsp() {
    for (slope, expect) in [(12u32, -12.30f32), (18, -18.13), (24, -24.10)] {
        let one_octave_below = response(cut(EqType::LowCut, 200.0, slope), 100.0);
        assert!((one_octave_below - expect).abs() < 1.0,
            "로우컷 {slope}dB/oct: 한 옥타브 아래 {one_octave_below:.2}dB (기대 {expect:.2})");
        let pass = response(cut(EqType::LowCut, 200.0, slope), 3_000.0);
        assert!(pass.abs() < 0.3, "로우컷 {slope}dB/oct가 통과대역(3kHz)을 {pass:.2}dB 건드린다");
    }
}

#[test]
fn high_cut_slope_reaches_the_dsp() {
    for (slope, expect) in [(12u32, -12.30f32), (18, -18.13), (24, -24.10)] {
        let one_octave_above = response(cut(EqType::HighCut, 2_000.0, slope), 4_000.0);
        assert!((one_octave_above - expect).abs() < 1.0,
            "하이컷 {slope}dB/oct: 한 옥타브 위 {one_octave_above:.2}dB (기대 {expect:.2})");
        let pass = response(cut(EqType::HighCut, 2_000.0, slope), 100.0);
        assert!(pass.abs() < 0.3, "하이컷 {slope}dB/oct가 통과대역(100Hz)을 {pass:.2}dB 건드린다");
    }
}

#[test]
fn changing_slope_does_not_step_the_waveform() {
    // 자동 EQ는 스피커가 벽에 가까워지면 드래그 도중 슬로프를 12 -> 18 -> 24로 바꾼다.
    let mut d = ChannelDspState::new();
    d.update_eq_targets(&[cut(EqType::LowCut, 150.0, 12)], FS);
    let mut ph = 0.0f32;
    let mut prev = 0.0f32;
    let (mut steady, mut transition) = (0.0f32, 0.0f32);
    for n in 0..72_000 {
        if n == 36_000 { d.update_eq_targets(&[cut(EqType::LowCut, 150.0, 24)], FS); }
        let y = d.process(ph.sin() * 0.2, FS);
        ph += 2.0 * std::f32::consts::PI * 440.0 / FS;
        let j = (y - prev).abs();
        prev = y;
        if (24_000..36_000).contains(&n) { steady = steady.max(j); }
        if (36_000..40_000).contains(&n) { transition = transition.max(j); }
    }
    assert!(transition <= steady * 1.10,
        "슬로프를 바꾸는 순간 파형이 튄다: 정상 {steady:.5} / 전환 {transition:.5}");
}
