//! 수동 진단: 테마 1의 세 번째 트랙(4채널 군중 녹음)을 바이노럴 끄고 틀 때 무엇이 걸리는가.
//!
//! 사용자 보고(2026-09-28): 바이노럴을 껐는데 스피커 위치에 따라 소리 위치가 조금씩 움직이고,
//! 같은 파일을 퀵타임으로 틀 때와 소리 나는 위치가 다르다 — 오디오 시스템 문제가 아닌가?
//!
//! 자동 테스트가 아니다(#[ignore]). 실행:
//!   DIAG_MULTI="<4채널 파일 경로>" [DIAG_TUNING_JSON=<앱 tuning_state JSON>] \
//!   cargo test --release --test diag_theme1_multitrack -- --ignored --nocapture --test-threads=1

use rust_lib_atmos_mixer_pro::audio::bass_route::compute_bass_route;
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::common::config::{EqBand, EqType, Point3D, RoomZone};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Arc;

const FS: u32 = 48_000;
const CH: usize = 12;
const BLOCK: usize = 512;
const ROOM: u32 = 1;

fn p(x: f32, y: f32, z: f32) -> Point3D {
    Point3D { x, y, z, ..Default::default() }
}

fn file() -> Arc<SoundData> {
    let path = std::env::var("DIAG_MULTI").expect("DIAG_MULTI=<4채널 파일 경로>");
    Arc::new(SoundData::load_from_file(std::path::Path::new(&path), FS).expect("파일 디코딩"))
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

/// 4채널 파일을 CH1부터 N:N으로 트는 믹서(앱의 "스테레오 켬 + CH1" 라우팅). 바이노럴 끔.
/// `app`이면 지금 앱 상태(CH1 서브, 크로스오버 80Hz, 채널 튜닝)를 건다.
fn mixer(data: Arc<SoundData>, app: bool) -> AudioMixer {
    let (gc_tx, _gc_rx) = crossbeam_channel::unbounded();
    let mut m = AudioMixer::new(FS, CH, BLOCK, gc_tx, None);
    for ch in 0..CH {
        GLOBAL_STATE.enabled_channels[ch].store(true, Ordering::SeqCst);
    }
    m.startup_ramp.current_gain = 1.0;
    let (sr, chans) = (data.sample_rate, data.channels);
    let mut inst = SoundInstance::new(
        1, 1, 1, "multi".to_string(), Some(data), None, sr, chans, false, 1.0, 0, true, None,
        GLOBAL_STATE.enabled_channels.len(),
    );
    inst.fade_weight = 1.0;
    m.instances[0] = Some(inst);

    // 지금 배치: CH1(서브)·CH2는 테마 1, CH3은 다른 방, CH4는 스피커 없음.
    m.room_zones = vec![RoomZone {
        room_id: ROOM,
        boundary_min: p(0.0, 0.0, 0.0),
        boundary_max: p(15.0, 20.0, 15.0),
        ear_level: 1.6,
        ..Default::default()
    }];
    m.listener_position = Some(p(7.5, 10.0, 1.6));
    m.channel_positions = vec![None; CH];
    m.channel_room_ids = vec![None; CH];
    m.channel_positions[0] = Some(p(8.74, 0.20, 2.61));
    m.channel_positions[1] = Some(p(13.03, 15.03, 2.44));
    m.channel_room_ids[0] = Some(ROOM);
    m.channel_room_ids[1] = Some(ROOM);
    m.binaural.enabled = false;

    if app {
        let is_sub: Vec<bool> = (0..CH).map(|c| c == 0).collect();
        let has_speaker: Vec<bool> = m.channel_positions.iter().map(|p| p.is_some()).collect();
        let route = compute_bass_route(&is_sub, &m.channel_room_ids, &has_speaker);
        let _ = m.set_bass_routing(route, is_sub);
        for c in m.crossovers.iter_mut() {
            c.set_target_freq(80.0, FS as f32);
        }
        for (ch, key) in [(0usize, "1"), (1, "2")] {
            if let Some((gain, delay, invert, bands)) = app_tuning(key) {
                let dsp = &mut m.channel_dsp[ch];
                dsp.update_delay_target(delay);
                dsp.update_eq_targets(&bands, FS as f32);
                dsp.phase_invert = invert;
                dsp.set_gain_db(gain);
            }
        }
    }
    m
}

/// 출력 CH1~CH4를 `secs`초 모은다.
fn render(m: &mut AudioMixer, secs: f32) -> [Vec<f32>; 4] {
    let blocks = (secs * FS as f32) as usize / BLOCK;
    let mut buf = vec![0.0f32; CH * BLOCK];
    let mut out: [Vec<f32>; 4] = Default::default();
    for _ in 0..blocks {
        buf.fill(0.0);
        m.process(&mut buf, CH);
        for f in 0..BLOCK {
            for (c, o) in out.iter_mut().enumerate() {
                o.push(buf[f * CH + c]);
            }
        }
    }
    out
}

/// 출력이 입력보다 몇 샘플 늦은가(0~2000에서 상관이 가장 큰 지연)와 그 지연에서의 (출력/입력 레벨dB, 차이 레벨dB).
fn compare(input: &[f32], output: &[f32]) -> (usize, f64, f64) {
    let n = input.len().min(output.len()).min(FS as usize * 8);
    let (a, b) = (&input[..n], &output[..n]);
    let lag_max = 2000usize;
    let mut best = (0usize, f64::MIN);
    for lag in 0..lag_max {
        let acc: f64 = (lag_max..n).map(|i| a[i - lag] as f64 * b[i] as f64).sum();
        if acc > best.1 {
            best = (lag, acc);
        }
    }
    let lag = best.0;
    let (mut ein, mut eout, mut ediff) = (0.0f64, 0.0f64, 0.0f64);
    for i in lag_max..n {
        let (x, y) = (a[i - lag] as f64, b[i] as f64);
        ein += x * x;
        eout += y * y;
        ediff += (y - x) * (y - x);
    }
    let db = |v: f64| 10.0 * v.max(1e-30).log10();
    (lag, db(eout / ein), db(ediff / ein))
}

/// 대역 [lo, hi)의 에너지 비(출력/입력, dB). 긴 FFT 한 번(분석용).
fn band_ratio_db(input: &[f32], output: &[f32], lo: f32, hi: f32) -> f64 {
    use realfft::RealFftPlanner;
    let n = (FS as usize * 8).min(input.len()).min(output.len());
    let energy = |x: &[f32]| {
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(n);
        let mut buf = x[..n].to_vec();
        let mut spec = fft.make_output_vec();
        fft.process(&mut buf, &mut spec).expect("fft");
        let bin = FS as f32 / n as f32;
        spec.iter()
            .enumerate()
            .filter(|(i, _)| {
                let f = *i as f32 * bin;
                f >= lo && f < hi
            })
            .map(|(_, c)| c.norm_sqr() as f64)
            .sum::<f64>()
    };
    10.0 * (energy(output) / energy(input).max(1e-30)).max(1e-30).log10()
}

#[test]
#[ignore]
fn 진단_멀티트랙_엔진_투명성과_지금_설정의_처리() {
    let data = file();
    let chans = data.channels as usize;
    let file_ch: Vec<Vec<f32>> =
        (0..chans).map(|c| data.samples.iter().skip(c).step_by(chans).copied().collect()).collect();
    println!("\n파일: {}채널 {}Hz, {:.1}초", chans, data.sample_rate, file_ch[0].len() as f32 / FS as f32);

    println!("\n[A] 효과 전부 끔(서브·자동 FX·반사·리버브 없음) — 엔진 경로만");
    let out = render(&mut mixer(data.clone(), false), 10.0);
    for c in 0..4.min(chans) {
        let (lag, level, diff) = compare(&file_ch[c], &out[c]);
        println!(
            "  파일 {}번 채널 → 출력 CH{}: 지연 {:.2}ms, 레벨 {:+.2}dB, 원본과의 차이 {:+.1}dB",
            c + 1,
            c + 1,
            lag as f32 * 1000.0 / FS as f32,
            level,
            diff
        );
    }

    println!("\n[B] 지금 앱 설정(CH1 서브·크로스오버 80Hz·CH1/CH2 자동 튜닝)");
    let out = render(&mut mixer(data, true), 10.0);
    for c in 0..4.min(chans) {
        let (lag, level, _) = compare(&file_ch[c], &out[c]);
        let bands: Vec<String> = [(20.0, 80.0), (80.0, 250.0), (250.0, 2000.0), (2000.0, 16000.0)]
            .iter()
            .map(|&(lo, hi)| format!("{lo:.0}-{hi:.0}Hz {:+.1}dB", band_ratio_db(&file_ch[c], &out[c], lo, hi)))
            .collect();
        println!(
            "  파일 {}번 → CH{}: 지연 {:.2}ms, 전체 {:+.1}dB | {}",
            c + 1,
            c + 1,
            lag as f32 * 1000.0 / FS as f32,
            level,
            bands.join(", ")
        );
    }
}
