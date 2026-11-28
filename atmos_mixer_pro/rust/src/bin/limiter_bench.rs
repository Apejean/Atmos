//! 리미터 몫 측정(설계 `docs/superpowers/specs/2026-11-27-limiter-silence-skip-design.md` 4절 1단계). 조사용이다(자동 시험 아님).
//!
//!   cargo run --release --bin limiter_bench -- [반복 횟수]
//!
//! 블록 1024프레임, 48kHz. 믹서 블록 하나의 처리 시간(평균·최대, µs)을 리미터 켬·끔으로 잰다.
//! 소리 나는 채널 12개(스테레오 6트랙 → CH1~12), 그 채널마다 EQ 4밴드, CH1~3 리버브 센드 0.5(현장과 비슷하게).
//! 폭 16(현장 평소)과 94(출력 설정이 비었거나 94채널을 다 연 경우)를 본다.
//! 끝으로 무음 채널 하나가 블록마다 쓰는 시간을 부분별(리미터·채널 DSP·크로스오버)로 잰다.

use rust_lib_atmos_mixer_pro::audio::crossover::LinkwitzRiley24;
use rust_lib_atmos_mixer_pro::audio::dsp::dsp_utils::ChannelDspState;
use rust_lib_atmos_mixer_pro::audio::limiter::PeakLimiter;
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::common::config::{EqBand, EqType};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::hint::black_box;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

const FS: u32 = 48_000;
const BLOCK: usize = 1024;
const FRAMES: usize = 48_000; // 1초 루프
const PLAYING: usize = 12;

fn data(channels: usize) -> Arc<SoundData> {
    let mut samples = Vec::with_capacity(FRAMES * channels);
    for i in 0..FRAMES {
        for c in 0..channels {
            samples.push(0.1 * ((i as f32) * 0.01 * (c as f32 + 1.0)).sin());
        }
    }
    Arc::new(SoundData { samples, channels: channels as u16, sample_rate: FS })
}

fn bands() -> Vec<EqBand> {
    let band = |filter_type, freq, gain, q_factor| EqBand {
        enabled: true,
        freq,
        gain,
        q_factor,
        filter_type,
        slope_db_per_oct: 12,
    };
    vec![
        band(EqType::LowCut, 40.0, 0.0, 0.707),
        band(EqType::Bell, 250.0, -3.0, 1.4),
        band(EqType::Bell, 2_500.0, 2.0, 1.0),
        band(EqType::HighShelf, 8_000.0, -2.0, 0.707),
    ]
}

fn build(width: usize, limiter: bool, skip: bool) -> AudioMixer {
    for (i, slot) in GLOBAL_STATE.enabled_channels.iter().enumerate() {
        slot.store(i < width, Ordering::SeqCst);
    }
    let (gc_tx, _gc_rx) = crossbeam_channel::bounded(4096);
    let mut m = AudioMixer::new(FS, width, BLOCK, gc_tx, None);
    m.startup_ramp.current_gain = 1.0;
    m.peak_limiter_enabled = limiter;
    m.limiter_skip = skip;
    for k in 0..PLAYING / 2 {
        let d = data(2);
        m.instances[k] = Some(SoundInstance::new(
            k as u64 + 1, k as u32 + 1, 1, format!("t{k}"), Some(d), None, FS, 2, true, 1.0, k * 2, true, None,
            GLOBAL_STATE.enabled_channels.len(),
        ));
    }
    let eq = bands();
    for ch in 0..PLAYING {
        m.channel_dsp[ch].update_eq_targets(&eq, FS as f32);
    }
    for ch in 0..3 {
        m.channel_dsp[ch].target_reverb_send = 0.5;
    }
    m
}

/// 반환: (평균 µs, 최대 µs)
fn run(width: usize, limiter: bool, skip: bool, iterations: usize) -> (f64, u128) {
    let mut m = build(width, limiter, skip);
    let mut out = vec![0.0f32; BLOCK * width];
    // EQ·리버브 스무딩이 자리 잡게 1초 남짓 먼저 돌린다.
    for _ in 0..200 {
        out.fill(0.0);
        m.process(&mut out, width);
    }
    let mut total = 0u128;
    let mut worst = 0u128;
    for _ in 0..iterations {
        out.fill(0.0);
        let t = Instant::now();
        m.process(&mut out, width);
        let us = t.elapsed().as_micros();
        total += us;
        worst = worst.max(us);
    }
    black_box(&out);
    (total as f64 / iterations as f64, worst)
}

/// 무음 채널 하나가 블록 하나에 쓰는 시간(µs). 입력은 모두 0.
fn silent_parts(blocks: usize) {
    let fs = FS as f32;
    let mut lim = PeakLimiter::new(fs, 1.0, 500.0, 0.99);
    let t = Instant::now();
    for _ in 0..blocks {
        for _ in 0..BLOCK {
            black_box(lim.process(black_box(0.0)));
        }
    }
    let lim_us = t.elapsed().as_secs_f64() * 1e6 / blocks as f64;

    let mut dsp = ChannelDspState::new();
    let t = Instant::now();
    for _ in 0..blocks {
        for _ in 0..BLOCK {
            black_box(dsp.process(black_box(0.0), fs));
        }
    }
    let dsp_us = t.elapsed().as_secs_f64() * 1e6 / blocks as f64;

    let mut dsp_eq = ChannelDspState::new();
    dsp_eq.update_eq_targets(&bands(), fs);
    dsp_eq.target_reverb_send = 0.5;
    for _ in 0..200 * BLOCK {
        dsp_eq.process(0.0, fs);
    }
    let t = Instant::now();
    for _ in 0..blocks {
        for _ in 0..BLOCK {
            black_box(dsp_eq.process(black_box(0.0), fs));
        }
    }
    let dsp_eq_us = t.elapsed().as_secs_f64() * 1e6 / blocks as f64;

    let mut xo = LinkwitzRiley24::new();
    xo.set_crossover_freq(80.0, fs);
    let t = Instant::now();
    for _ in 0..blocks {
        for _ in 0..BLOCK {
            black_box(xo.split(black_box(0.0)));
        }
    }
    let xo_us = t.elapsed().as_secs_f64() * 1e6 / blocks as f64;

    println!(
        "무음 채널 하나, 블록 하나: 리미터 {lim_us:.1}µs · 채널 DSP 기본 {dsp_us:.1}µs · 채널 DSP(EQ 4밴드+리버브 0.5) {dsp_eq_us:.1}µs · 크로스오버 {xo_us:.1}µs"
    );
}

fn main() {
    // 실제 콜백(engine.rs)처럼 비정규수를 0으로 다룬다. 없으면 무음으로 줄어드는 필터가 비정규수에서 느려져 실제와 다르다.
    let _denormals = rust_lib_atmos_mixer_pro::audio::denormal::DenormalGuard::new();
    let iterations: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(2000);
    let budget = BLOCK as f64 / FS as f64 * 1e6;
    println!("블록 {BLOCK}프레임 · {FS}Hz · 예산 {budget:.0}µs · 소리 나는 채널 {PLAYING}개 · 반복 {iterations}");
    for width in [16usize, 94] {
        let (on_avg, on_max) = run(width, true, false, iterations);
        let (skip_avg, skip_max) = run(width, true, true, iterations);
        let (off_avg, off_max) = run(width, false, false, iterations);
        let share = (on_avg - off_avg) / on_avg * 100.0;
        println!(
            "폭 {width}: 리미터 켬(건너뛰기 끔) 평균 {on_avg:.1}µs·최대 {on_max}µs / 켬(건너뛰기 켬) 평균 {skip_avg:.1}µs·최대 {skip_max}µs / 리미터 끔 평균 {off_avg:.1}µs·최대 {off_max}µs / 리미터 몫 {:.1}µs({share:.1}%) → 건너뛰기로 {:.1}µs 줄어듦",
            on_avg - off_avg,
            on_avg - skip_avg
        );
    }
    silent_parts(iterations);
}
