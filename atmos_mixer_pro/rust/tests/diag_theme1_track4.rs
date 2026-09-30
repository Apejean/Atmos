//! 수동 진단: 테마 1에서 CH1을 서브로, 저음이 강한 네 번째 트랙을 CH2로 튼 현장 장면.
//!
//! 사용자 보고(2026-09-28):
//!   5-1 서브 소리가 CH1 자리가 아니라 CH2 자리에서 들린다.
//!   5-2 CH2를 옮기면 FX가 위치를 따라가는지 네 번째 트랙으로 확인.
//!   5-3 크로스오버 아래 저역은 CH1에서만 나와야 하는데 CH2에서도 나온다.
//!
//! 자동 테스트가 아니다(전부 #[ignore]). 실행:
//!   DIAG_TRACK="<트랙 경로>" [DIAG_LAYOUT=<앱 exhibition_speaker_layout JSON>]
//!   [DIAG_TUNING_JSON=<앱 tuning_state JSON>] [DIAG_XOVER=80] \
//!   cargo test --test diag_theme1_track4 -- --ignored --nocapture --test-threads=1

use realfft::RealFftPlanner;
use rust_lib_atmos_mixer_pro::audio::bass_route::compute_bass_route;
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::common::config::{EqBand, EqType, Point3D, RoomZone};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use rustfft::num_complex::Complex;
use std::sync::atomic::Ordering;
use std::sync::Arc;

const FS: u32 = 48_000;
const CH: usize = 12;
const BLOCK: usize = 512;
const SUB: usize = 0; // 화면 CH1
const MAIN: usize = 1; // 화면 CH2
const ROOM: u32 = 1;
/// 테마 1 청취 지점(방 중심, 귀 높이).
const EAR: (f32, f32, f32) = (7.5, 10.0, 1.6);
/// 앱 자동 게인의 기준 거리(방 짧은 변의 절반, Dart gainRefDistance).
const GAIN_REF_M: f32 = 7.5;
const SPEED_OF_SOUND: f32 = 343.0;

const BANDS: [(f32, f32); 8] = [
    (20.0, 40.0),
    (40.0, 60.0),
    (60.0, 80.0),
    (80.0, 100.0),
    (100.0, 150.0),
    (150.0, 250.0),
    (250.0, 1000.0),
    (1000.0, 16000.0),
];

fn p(x: f32, y: f32, z: f32) -> Point3D {
    Point3D { x, y, z, ..Default::default() }
}

fn ear() -> Point3D {
    p(EAR.0, EAR.1, EAR.2)
}

fn dist(a: &Point3D, b: &Point3D) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

fn db(x: f64) -> f64 {
    10.0 * x.max(1e-30).log10()
}

/// 테마 1: 15 x 20m, 천장 15m, 귀 높이 1.6m, 콘크리트(흡음 0.018).
fn zone() -> RoomZone {
    RoomZone {
        room_id: ROOM,
        boundary_min: p(0.0, 0.0, 0.0),
        boundary_max: p(15.0, 20.0, 15.0),
        ear_level: EAR.2,
        absorption_coeff: 0.0183,
        ..Default::default()
    }
}

/// (CH1 서브 좌표, CH2 좌표). DIAG_LAYOUT이 있으면 앱 저장본에서 읽고, 없으면 19:56 저장본 좌표.
fn layout() -> (Point3D, Point3D) {
    let fallback = (p(10.74, 15.26, 2.61), p(4.56, 14.38, 14.75));
    let Ok(path) = std::env::var("DIAG_LAYOUT") else { return fallback };
    let text = std::fs::read_to_string(path).expect("DIAG_LAYOUT 파일");
    let list: serde_json::Value = serde_json::from_str(&text).expect("DIAG_LAYOUT JSON");
    let find = |ch: u64| {
        list.as_array()?
            .iter()
            .find(|s| s.get("channel").and_then(|v| v.as_u64()) == Some(ch))
            .map(|s| {
                let f = |k: &str| s.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                p(f("x"), f("y"), f("height_z"))
            })
    };
    (find(0).unwrap_or(fallback.0), find(1).unwrap_or(fallback.1))
}

fn crossover_hz() -> f32 {
    std::env::var("DIAG_XOVER").ok().and_then(|v| v.parse().ok()).unwrap_or(80.0)
}

/// 트랙을 이 초부터 튼다(네 번째 트랙은 앞 12초가 저역 없는 인트로다).
fn start_s() -> f32 {
    std::env::var("DIAG_START").ok().and_then(|v| v.parse().ok()).unwrap_or(16.0)
}

fn track() -> Arc<SoundData> {
    let path = std::env::var("DIAG_TRACK").expect("DIAG_TRACK=<트랙 경로>");
    Arc::new(SoundData::load_from_file(std::path::Path::new(&path), FS).expect("트랙 디코딩"))
}

/// 고정 시드 백색잡음(방향 기준 측정용).
fn noise() -> Arc<SoundData> {
    let n = FS as usize * 8;
    let mut seed: u32 = 12_345;
    let samples: Vec<f32> = (0..n)
        .map(|_| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            ((seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5) * 0.2
        })
        .collect();
    Arc::new(SoundData { samples, channels: 1, sample_rate: FS })
}

fn tone(freq: f32) -> Arc<SoundData> {
    let n = FS as usize * 8;
    let samples: Vec<f32> =
        (0..n).map(|i| 0.1 * (std::f32::consts::TAU * freq * i as f32 / FS as f32).sin()).collect();
    Arc::new(SoundData { samples, channels: 1, sample_rate: FS })
}

#[derive(Clone, Copy)]
enum Fx {
    /// 게인 0dB·지연 0ms·EQ 없음 — 크로스오버 분리만 본다.
    Flat,
    /// 앱과 같은 식의 시간 정렬 지연·거리 게인(DIAG_TUNING_JSON이 있으면 앱 튜닝 그대로).
    App,
}

#[derive(Clone, Copy)]
struct Setup {
    binaural: bool,
    sub_on: bool,
    fx: Fx,
}

/// 앱 tuning_state의 한 채널 → (게인dB, 딜레이ms, 극성 반전, EQ 밴드).
fn app_tuning(key: &str) -> Option<(f32, f32, bool, Vec<EqBand>)> {
    let path = std::env::var("DIAG_TUNING_JSON").ok()?;
    let t: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    let c = t.get(key)?;
    let f = |k: &str, i: usize| c.get(k)?.get(i)?.as_f64().map(|v| v as f32);
    let types = [EqType::LowCut, EqType::LowShelf, EqType::Bell, EqType::Notch, EqType::HighShelf, EqType::HighCut];
    let mut bands = Vec::new();
    for i in 0..8 {
        let ty = c.get("bandTypes").and_then(|v| v.get(i)).and_then(|v| v.as_u64()).unwrap_or(2) as usize;
        bands.push(EqBand {
            enabled: c.get("bandEnabled")?.get(i)?.as_bool()?,
            freq: f("freqs", i)?,
            gain: f("gains", i)?,
            q_factor: f("qs", i)?,
            filter_type: types[ty.min(5)],
            slope_db_per_oct: c.get("bandSlopes").and_then(|v| v.get(i)).and_then(|v| v.as_u64()).unwrap_or(12) as u32,
        });
    }
    Some((
        c.get("gainDb")?.as_f64()? as f32,
        c.get("delay")?.as_f64()? as f32,
        c.get("phaseInvert")?.as_bool()?,
        bands,
    ))
}

/// 앱 acoustic_sync와 같은 식: 가장 먼 스피커 기준 시간 정렬, 기준 거리 대비 거리 게인.
fn formula_fx(sub_pos: &Point3D, main_pos: &Point3D) -> [(f32, f32); 2] {
    let (d_sub, d_main) = (dist(sub_pos, &ear()), dist(main_pos, &ear()));
    let d_max = d_sub.max(d_main);
    let fx = |d: f32| {
        (
            (20.0 * (d / GAIN_REF_M).log10()).clamp(-24.0, 12.0),
            ((d_max - d) / SPEED_OF_SOUND * 1000.0).clamp(0.0, 50.0),
        )
    };
    [fx(d_sub), fx(d_main)]
}

fn mixer(s: Setup, data: Arc<SoundData>, out_ch: usize, sub_pos: &Point3D, main_pos: &Point3D) -> AudioMixer {
    let (gc_tx, _gc_rx) = crossbeam_channel::unbounded();
    let mut m = AudioMixer::new(FS, CH, BLOCK, gc_tx, None);
    for ch in 0..CH {
        GLOBAL_STATE.enabled_channels[ch].store(true, Ordering::SeqCst);
    }
    GLOBAL_STATE.hrtf_yaw.store(0.0f32.to_bits(), Ordering::Relaxed);
    m.startup_ramp.current_gain = 1.0;

    let (sr, chans) = (data.sample_rate, data.channels);
    let frames = data.samples.len() / (chans.max(1) as usize);
    let mut inst = SoundInstance::new(
        1, 1, 1, "diag".to_string(), Some(data), None, sr, chans, true, 1.0, out_ch, false, None,
        GLOBAL_STATE.enabled_channels.len(),
    );
    inst.fade_weight = 1.0;
    // 짧은 합성 신호(잡음·순음)는 처음부터, 긴 트랙은 start_s()부터 튼다.
    let start = (start_s() * sr as f32) as usize;
    if frames > start + sr as usize * 20 {
        inst.cursor = start as f64;
    }
    m.instances[0] = Some(inst);

    m.room_zones = vec![zone()];
    m.listener_position = Some(ear());
    m.channel_positions = vec![None; CH];
    m.channel_room_ids = vec![None; CH];
    m.channel_positions[SUB] = Some(sub_pos.clone());
    m.channel_positions[MAIN] = Some(main_pos.clone());
    m.channel_room_ids[SUB] = Some(ROOM);
    m.channel_room_ids[MAIN] = Some(ROOM);
    m.binaural.enabled = s.binaural;
    m.set_binaural_room(Some(ROOM));
    m.recalculate_binaural_channel_azimuths();

    let is_sub: Vec<bool> = (0..CH).map(|c| s.sub_on && c == SUB).collect();
    let has_speaker: Vec<bool> = m.channel_positions.iter().map(|p| p.is_some()).collect();
    let route = compute_bass_route(&is_sub, &m.channel_room_ids, &has_speaker);
    let _ = m.set_bass_routing(route, is_sub);
    let xover = crossover_hz();
    for c in m.crossovers.iter_mut() {
        c.set_target_freq(xover, FS as f32);
    }

    if let Fx::App = s.fx {
        let formula = formula_fx(sub_pos, main_pos);
        for (i, (ch, key)) in [(SUB, "1"), (MAIN, "2")].into_iter().enumerate() {
            let (gain, delay, invert, bands) =
                app_tuning(key).unwrap_or((formula[i].0, formula[i].1, false, Vec::new()));
            let dsp = &mut m.channel_dsp[ch];
            dsp.update_delay_target(delay);
            dsp.update_eq_targets(&bands, FS as f32);
            dsp.phase_invert = invert;
            dsp.set_gain_db(gain);
        }
    }
    m
}

/// `secs`초 돌리고 앞 `skip`초를 버린 (출력 0, 출력 1). 바이노럴이면 헤드폰 L/R, 아니면 CH1/CH2.
fn render(m: &mut AudioMixer, secs: f32, skip: f32) -> (Vec<f32>, Vec<f32>) {
    let blocks = (secs * FS as f32) as usize / BLOCK;
    let skip_blocks = (skip * FS as f32) as usize / BLOCK;
    let mut buf = vec![0.0f32; CH * BLOCK];
    let (mut a, mut b) = (Vec::new(), Vec::new());
    for i in 0..blocks {
        buf.fill(0.0);
        m.process(&mut buf, CH);
        if i >= skip_blocks {
            for f in 0..BLOCK {
                a.push(buf[f * CH]);
                b.push(buf[f * CH + 1]);
            }
        }
    }
    (a, b)
}

/// 대역별 파워(Welch, 8192점 한 창, 50% 겹침).
fn band_powers(x: &[f32]) -> [f64; BANDS.len()] {
    const N: usize = 8192;
    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(N);
    let window: Vec<f32> =
        (0..N).map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / N as f32).cos()).collect();
    let mut acc = vec![0.0f64; N / 2 + 1];
    let (mut input, mut spec) = (fft.make_input_vec(), fft.make_output_vec());
    let mut start = 0;
    while start + N <= x.len() {
        for i in 0..N {
            input[i] = x[start + i] * window[i];
        }
        fft.process(&mut input, &mut spec).expect("fft");
        for (a, c) in acc.iter_mut().zip(&spec) {
            *a += c.norm_sqr() as f64;
        }
        start += N / 2;
    }
    let bin_hz = FS as f32 / N as f32;
    let mut out = [0.0f64; BANDS.len()];
    for (k, &(lo, hi)) in BANDS.iter().enumerate() {
        out[k] = acc
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                let f = *i as f32 * bin_hz;
                f >= lo && f < hi
            })
            .map(|(_, v)| *v)
            .sum();
    }
    out
}

/// FFT로 [lo, hi) 대역만 남긴다(분석용, 실시간 아님).
fn bandpass(x: &[f32], lo: f32, hi: f32) -> Vec<f32> {
    let n = x.len().next_power_of_two();
    let mut planner = RealFftPlanner::<f32>::new();
    let fwd = planner.plan_fft_forward(n);
    let inv = planner.plan_fft_inverse(n);
    let mut input = fwd.make_input_vec();
    input[..x.len()].copy_from_slice(x);
    let mut spec = fwd.make_output_vec();
    fwd.process(&mut input, &mut spec).expect("fft");
    let bin_hz = FS as f32 / n as f32;
    for (i, c) in spec.iter_mut().enumerate() {
        let f = i as f32 * bin_hz;
        if f < lo || f >= hi {
            *c = Complex::new(0.0, 0.0);
        }
    }
    let mut out = inv.make_output_vec();
    inv.process(&mut spec, &mut out).expect("ifft");
    out.truncate(x.len());
    let scale = 1.0 / n as f32;
    out.iter_mut().for_each(|v| *v *= scale);
    out
}

/// 두 귀 도달 시간차(ms, +면 왼쪽이 먼저)·상관계수·레벨차(dB, +면 왼쪽이 큼). ±1.2ms에서 찾는다.
fn interaural(l: &[f32], r: &[f32]) -> (f32, f32, f64) {
    let max_lag = (0.0012 * FS as f32) as isize;
    let n = l.len().min(r.len()) as isize;
    let el: f64 = l.iter().map(|v| (*v as f64).powi(2)).sum();
    let er: f64 = r.iter().map(|v| (*v as f64).powi(2)).sum();
    let norm = (el * er).sqrt().max(1e-30);
    let mut best = (0isize, f64::MIN);
    for lag in -max_lag..=max_lag {
        let mut acc = 0.0f64;
        for i in max_lag..(n - max_lag) {
            acc += l[i as usize] as f64 * r[(i + lag) as usize] as f64;
        }
        if acc > best.1 {
            best = (lag, acc);
        }
    }
    (best.0 as f32 * 1000.0 / FS as f32, (best.1 / norm) as f32, db(el / er.max(1e-30)))
}

const ITD_BANDS: [(f32, f32); 3] = [(25.0, 80.0), (80.0, 250.0), (500.0, 1500.0)];

fn itd_row(l: &[f32], r: &[f32]) -> String {
    ITD_BANDS
        .iter()
        .map(|&(lo, hi)| {
            let (t, c, ild) = interaural(&bandpass(l, lo, hi), &bandpass(r, lo, hi));
            format!("{t:+.2}ms {ild:+.1}dB (r={c:.2})")
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

#[test]
#[ignore]
fn 진단_5_3_크로스오버_대역별_출력() {
    let (sub_pos, main_pos) = layout();
    let data = track();
    let mono: Vec<f32> = match data.channels {
        1 => data.samples.clone(),
        c => data.samples.chunks(c as usize).map(|f| f.iter().sum::<f32>() / c as f32).collect(),
    };
    // 렌더와 같은 구간(시작 + 2~14초)의 원음
    let s0 = ((start_s() + 2.0) * FS as f32) as usize;
    let src = band_powers(&mono[s0.min(mono.len())..(s0 + FS as usize * 12).min(mono.len())]);
    println!(
        "\n[5-3] 네 번째 트랙({}초부터)을 CH2로, 크로스오버 {}Hz, 바이노럴 끔, FX 없음(분리만)",
        start_s(),
        crossover_hz()
    );
    for (label, sub_on) in [("CH1 서브 지정", true), ("서브 지정 해제(비교)", false)] {
        let mut m = mixer(Setup { binaural: false, sub_on, fx: Fx::Flat }, data.clone(), MAIN, &sub_pos, &main_pos);
        let (ch1, ch2) = render(&mut m, 14.0, 2.0);
        let (b1, b2) = (band_powers(&ch1), band_powers(&ch2));
        println!("  {label}\n    대역(Hz)   | 원음 대비 CH1 | 원음 대비 CH2 | CH1 몫");
        for (k, &(lo, hi)) in BANDS.iter().enumerate() {
            println!(
                "    {lo:>5.0}-{hi:<5.0} | {:>+8.1}dB   | {:>+8.1}dB   | {:>5.1}%",
                db(b1[k] / src[k]),
                db(b2[k] / src[k]),
                100.0 * b1[k] / (b1[k] + b2[k]).max(1e-30)
            );
        }
    }
}

#[test]
#[ignore]
fn 진단_5_1_헤드폰_대역별_방향() {
    let (sub_pos, main_pos) = layout();
    println!(
        "\n[5-1] 헤드폰(바이노럴) 두 귀 시간차·레벨차 — 대역: {:?}\n  CH1 {:?} {:.1}m / CH2 {:?} {:.1}m",
        ITD_BANDS,
        (sub_pos.x, sub_pos.y, sub_pos.z),
        dist(&sub_pos, &ear()),
        (main_pos.x, main_pos.y, main_pos.z),
        dist(&main_pos, &ear())
    );
    // 방향 기준: 서브 지정 없이 각 스피커에서만 백색잡음
    for (label, ch) in [("기준: CH1 자리 잡음", SUB), ("기준: CH2 자리 잡음", MAIN)] {
        let mut m = mixer(Setup { binaural: true, sub_on: false, fx: Fx::Flat }, noise(), ch, &sub_pos, &main_pos);
        let (l, r) = render(&mut m, 6.0, 1.0);
        println!("  {label:<22} {}", itd_row(&l, &r));
    }
    // 순음을 CH2로(서브 지정): 50Hz는 전부 서브로 가야 한다
    for f in [50.0, 150.0, 1000.0] {
        let mut m = mixer(Setup { binaural: true, sub_on: true, fx: Fx::App }, tone(f), MAIN, &sub_pos, &main_pos);
        let (l, r) = render(&mut m, 6.0, 1.0);
        let (t, c, ild) = interaural(&l, &r);
        println!("  {f:>5.0}Hz 순음 → CH2 (서브 켬)  {t:+.2}ms {ild:+.1}dB (r={c:.2})");
    }
    // 실제 트랙(앱 FX)
    let data = track();
    let mut m = mixer(Setup { binaural: true, sub_on: true, fx: Fx::App }, data.clone(), MAIN, &sub_pos, &main_pos);
    let (l, r) = render(&mut m, 12.0, 2.0);
    println!("  네 번째 트랙(서브 켬)   {}", itd_row(&l, &r));

    // 헤드폰에 들어간 대역별 에너지가 CH1·CH2 중 어디서 왔나(한쪽만 렌더)
    let only = |keep: usize| {
        let mut m = mixer(Setup { binaural: true, sub_on: true, fx: Fx::App }, data.clone(), MAIN, &sub_pos, &main_pos);
        for (ch, slot) in m.binaural.channel_enabled_mut().iter_mut().enumerate() {
            *slot = ch == keep;
        }
        let (l, r) = render(&mut m, 12.0, 2.0);
        let (bl, br) = (band_powers(&l), band_powers(&r));
        let mut out = [0.0f64; BANDS.len()];
        for k in 0..BANDS.len() {
            out[k] = bl[k] + br[k];
        }
        out
    };
    let (from_sub, from_main) = (only(SUB), only(MAIN));
    println!("  헤드폰 대역별 출처(트랙): 대역 | CH1(서브) 몫");
    for (k, &(lo, hi)) in BANDS.iter().enumerate() {
        println!("    {lo:>5.0}-{hi:<5.0} | {:>5.1}%", 100.0 * from_sub[k] / (from_sub[k] + from_main[k]).max(1e-30));
    }
}

#[test]
#[ignore]
fn 진단_5_2_ch2_위치별_청취점() {
    let (sub_pos, main_now) = layout();
    let data = track();
    println!("\n[5-2] CH2를 옮기며 앱 식의 FX(시간 정렬·거리 게인)를 걸고 헤드폰에서 잰다(EQ·반사 제외)");
    println!("  CH2 자리(x,y,z)        거리  | CH2 게인·지연 | CH1 지연 | 헤드폰 전체(상대) | 전체 대비 <80Hz | 80-250 | 1k-16k");
    for main_pos in [main_now, p(7.5, 13.0, 2.0), p(1.0, 19.0, 3.0), p(13.0, 4.0, 8.0), p(4.5, 14.4, 6.0)] {
        let fx = formula_fx(&sub_pos, &main_pos);
        // 앱 튜닝 파일이 있어도 이 표는 자리마다 새로 계산한 FX로 본다(EQ 없음).
        let mut m = mixer(Setup { binaural: true, sub_on: true, fx: Fx::Flat }, data.clone(), MAIN, &sub_pos, &main_pos);
        for (ch, (gain, delay)) in [(SUB, fx[0]), (MAIN, fx[1])] {
            let dsp = &mut m.channel_dsp[ch];
            dsp.update_delay_target(delay);
            dsp.set_gain_db(gain);
        }
        let (l, r) = render(&mut m, 12.0, 2.0);
        let (bl, br) = (band_powers(&l), band_powers(&r));
        let sum = |ks: std::ops::Range<usize>| ks.map(|k| bl[k] + br[k]).sum::<f64>();
        let total = sum(0..BANDS.len());
        println!(
            "  ({:>5.2},{:>5.2},{:>5.2}) {:>5.1}m | {:>+5.1}dB {:>4.1}ms | {:>5.1}ms | {:>+7.1}dB | {:>+5.1} | {:>+6.1} | {:>+6.1}",
            main_pos.x,
            main_pos.y,
            main_pos.z,
            dist(&main_pos, &ear()),
            fx[1].0,
            fx[1].1,
            fx[0].1,
            db(total),
            db(sum(0..3) / total),
            db(sum(3..6) / total),
            db(sum(7..8) / total),
        );
    }
}
