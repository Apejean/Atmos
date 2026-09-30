//! HRTF 데이터셋 보정. 엔진이 시작할 때 1회 실행한다(오디오 스레드 밖).
//!
//! 앱이 쓰는 MIT KEMAR(mit_kemar_normal_pinna.sofa)는 보정되지 않은 원본 측정이다. 그대로
//! 쓰면 모든 방향에 공통인 색채가 헤드폰에 그대로 실린다.
//! - 측정용 스피커의 저역 부족: 정면 기준 40Hz가 중역보다 -21dB
//! - 더미 헤드 귓구멍 공진: 2~4kHz가 중역보다 +16dB(헤드폰으로 들으면 공진이 두 번 걸린다)
//! - 44.1kHz 데이터를 48kHz 엔진에서 그대로 쓰면 주파수와 도달 시간차가 8.8% 어긋난다
//!
//! 표준 처리를 한다.
//! 1. 엔진 샘플레이트로 리샘플링
//! 2. **확산음장 등화** — 모든 방향·양쪽 귀의 평균 파워(방향과 무관한 색채)를 구해 그 역을
//!    곱한다. 방향마다 달라지는 정보(좌우 레벨차, 귓바퀴 노치)는 남는다.
//! 3. **저역 확장** — 머리는 저역을 거의 가리지 않으므로 100Hz 아래는 평탄하게 둔다.
//! 4. **최소위상 + 도달 시간차 재구성** — 보정한 크기로 최소위상 IR을 만들고, 원래 귀별
//!    도달 시각만큼 지연을 다시 넣어 도달 시간차(ITD)를 그대로 유지한다.

use realfft::RealFftPlanner;
use rustfft::num_complex::Complex;
use sofa_reader::SofaFile;

/// 보정 곡선을 계산할 때 쓰는 확산음장 목표 레벨(dB). 보정 폭 상한(±[MAX_CORRECTION_DB])이
/// 이 값 기준으로 걸리므로, 곡선 모양을 바꾸지 않으려면 이 값은 그대로 둔다
/// (보정 전 정면 중역은 약 -11dB였다). 최종 음량은 [OUTPUT_MAKEUP_DB]가 정한다.
pub const DIFFUSE_FIELD_TARGET_DB: f32 = -6.0;
/// 보정 곡선 전체에 곱하는 고정 게인(dB) — 확산음장 평균을 표준 0dB로 올린다. 기준 거리
/// 정면 스피커가 헤드폰에서 원음과 거의 같은 크기로 들린다(백색잡음 두 귀 평균 약 0dB).
///
/// 예전에는 여러 스피커 합산 여유로 -6dB에 두었다. 미리듣기가 거리 감쇠를 흉내 내면서
/// 먼 스피커는 튜닝이 올려 둔 게인만큼 다시 작아지는데(테마 1 CH2 13.8m: -5dB) 여기에
/// -6dB까지 겹쳐 미리듣기 전체가 작아졌다(실기 보고: "전체적으로 소리가 너무 작아졌다").
/// 합산 피크는 출력단 리미터가 잡는다.
pub const OUTPUT_MAKEUP_DB: f32 = -DIFFUSE_FIELD_TARGET_DB;
/// 보정 후 IR 길이(샘플). 렌더러의 FFT 크기가 바뀌지 않도록 원래 길이로 둔다.
pub const EQUALIZED_IR_LENGTH: usize = 512;

/// 이 아래는 평탄하게 확장한다.
const LF_FLAT_BELOW_HZ: f32 = 100.0;
/// 평탄 구간과 보정 구간을 이 주파수까지 부드럽게 잇는다.
const LF_BLEND_UNTIL_HZ: f32 = 200.0;
/// 이 위(원본에 거의 내용이 없는 대역)는 이 주파수의 레벨로 평탄하게 둔다.
const HF_FLAT_ABOVE_HZ: f32 = 20_000.0;
/// 보정량 상한(dB). 측정 잡음이나 깊은 노치를 과하게 키우지 않는다.
const MAX_CORRECTION_DB: f32 = 15.0;
/// 고역 보정 구간. 이 위로는 확산음장을 1옥타브로 굵게 평활하고, 보정 폭을 1kHz 보정값
/// 기준 ±[HF_CORRECTION_RANGE_DB]로 제한한다(4~6kHz에서 부드럽게 전환).
///
/// 고역의 확산음장 특성은 방향과 개인차가 커서, 실무에서도 이 대역은 완만하게만 보정한다
/// (1/3옥타브로 그대로 뒤집으면 좁은 특징까지 과하게 뒤집힌다).
///
/// 참고: 보정 후 정면 음원은 8kHz 부근에 약 -15dB 골이 남는다. 이건 KEMAR 귓바퀴 노치 —
/// "앞에서 온다"는 방향 정보라서(원본의 정면/확산음장 비율 자체가 7k -9, 8k -15, 9k -6dB)
/// 없애지 않는다.
const HF_GENTLE_FROM_HZ: f32 = 4_000.0;
const HF_GENTLE_FULL_HZ: f32 = 6_000.0;
const HF_CORRECTION_RANGE_DB: f32 = 6.0;
/// 분석용 FFT 크기. 48kHz에서 빈 간격 약 11.7Hz.
const FFT_SIZE: usize = 4096;
/// 도달 시각을 잡는 문턱(각 IR 최대값 대비).
const ONSET_THRESHOLD: f32 = 0.1;
/// 끝부분 페이드아웃 길이(샘플). 자른 자리에서 불연속이 생기지 않게 한다.
const TAIL_FADE: usize = 48;

/// `db`의 모든 HRIR을 `target_fs`로 리샘플링하고 확산음장 등화·저역 확장을 적용한다.
pub fn equalize(db: &mut SofaFile, target_fs: f32) {
    let n_in = db.ir_length;
    let n_irs = db.num_measurements * 2;
    if n_in == 0 || n_irs == 0 || db.impulse_responses.len() < n_irs * n_in {
        return;
    }
    let fs_in = db.sample_rate;

    // 1. 리샘플링. 모든 IR이 길이·비율이 같으므로 커널을 한 번만 만들어 재사용한다
    //    (IR마다 삼각함수로 커널을 다시 계산하면 엔진 첫 기동이 1초 넘게 멈췄다).
    let kernels = resample_kernels(n_in, fs_in, target_fs);
    let resampled: Vec<Vec<f32>> = (0..n_irs)
        .map(|i| apply_kernels(&db.impulse_responses[i * n_in..(i + 1) * n_in], &kernels))
        .collect();

    // 2. 스펙트럼 크기와 귀별 도달 시각
    let mut planner = RealFftPlanner::<f32>::new();
    let r2c = planner.plan_fft_forward(FFT_SIZE);
    let c2r = planner.plan_fft_inverse(FFT_SIZE);
    let bins = FFT_SIZE / 2 + 1;
    let mut mags: Vec<Vec<f32>> = Vec::with_capacity(n_irs);
    let mut onsets: Vec<f32> = Vec::with_capacity(n_irs);
    let mut df_power = vec![0.0f64; bins];
    for ir in &resampled {
        let mut buf = vec![0.0f32; FFT_SIZE];
        let len = ir.len().min(FFT_SIZE);
        buf[..len].copy_from_slice(&ir[..len]);
        let mut spec = r2c.make_output_vec();
        let _ = r2c.process(&mut buf, &mut spec);
        let mag: Vec<f32> = spec.iter().map(|c| c.norm()).collect();
        for (p, m) in df_power.iter_mut().zip(&mag) {
            *p += (*m as f64) * (*m as f64);
        }
        mags.push(mag);
        onsets.push(onset(ir));
    }
    for p in df_power.iter_mut() {
        *p /= n_irs as f64;
    }

    // 3. 확산음장 응답(1/3옥타브 평활) -> 보정 곡선
    let bin_hz = target_fs / FFT_SIZE as f32;
    let df_third = smooth_fractional_octave(&df_power, bin_hz, 3.0);
    let df_octave = smooth_fractional_octave(&df_power, bin_hz, 1.0);
    let to_db = |x: f32| 20.0 * x.max(1e-9).log10();
    let full_db: Vec<f32> = df_third
        .iter()
        .map(|&d| (DIFFUSE_FIELD_TARGET_DB - to_db(d)).clamp(-MAX_CORRECTION_DB, MAX_CORRECTION_DB))
        .collect();
    // 고역용: 1옥타브 평활 + 1kHz 보정값 기준 ±HF_CORRECTION_RANGE_DB
    let k_1k = ((1_000.0 / bin_hz).round() as usize).min(bins - 1);
    let ref_db = full_db[k_1k];
    let gentle_db: Vec<f32> = df_octave
        .iter()
        .map(|&d| {
            (DIFFUSE_FIELD_TARGET_DB - to_db(d))
                .clamp(ref_db - HF_CORRECTION_RANGE_DB, ref_db + HF_CORRECTION_RANGE_DB)
        })
        .collect();
    let correction: Vec<f32> = (0..bins)
        .map(|k| {
            let f = k as f32 * bin_hz;
            let w = ((f - HF_GENTLE_FROM_HZ) / (HF_GENTLE_FULL_HZ - HF_GENTLE_FROM_HZ)).clamp(0.0, 1.0);
            let w = 0.5 - 0.5 * (std::f32::consts::PI * w).cos();
            let db = (1.0 - w) * full_db[k] + w * gentle_db[k];
            10.0f32.powf((db + OUTPUT_MAKEUP_DB) / 20.0)
        })
        .collect();

    // 4. IR마다 보정 크기 -> 최소위상 -> 도달 시각 지연
    let k_flat = ((LF_FLAT_BELOW_HZ / bin_hz) as usize).max(1);
    let k_blend = ((LF_BLEND_UNTIL_HZ / bin_hz) as usize).max(k_flat + 1);
    let k_top = ((HF_FLAT_ABOVE_HZ / bin_hz) as usize).min(bins - 1);
    let mut out = vec![0.0f32; n_irs * EQUALIZED_IR_LENGTH];
    for (i, mag) in mags.iter().enumerate() {
        let mut m: Vec<f32> = mag.iter().zip(&correction).map(|(a, c)| a * c).collect();

        // 저역: 200Hz 레벨로 평탄하게(100~200Hz는 로그 크기로 부드럽게 잇는다).
        let anchor = m[k_blend].max(1e-9).ln();
        for (k, v) in m.iter_mut().enumerate().take(k_blend) {
            let w = if k <= k_flat {
                0.0
            } else {
                let x = (k - k_flat) as f32 / (k_blend - k_flat) as f32;
                0.5 - 0.5 * (std::f32::consts::PI * x).cos()
            };
            *v = (w * v.max(1e-9).ln() + (1.0 - w) * anchor).exp();
        }
        // 20kHz 위: 그 지점 레벨로 평탄하게.
        let top = m[k_top];
        for v in m.iter_mut().skip(k_top + 1) {
            *v = top;
        }

        let h = min_phase_with_delay(&m, onsets[i], &*r2c, &*c2r);
        let dst = &mut out[i * EQUALIZED_IR_LENGTH..(i + 1) * EQUALIZED_IR_LENGTH];
        dst.copy_from_slice(&h[..EQUALIZED_IR_LENGTH]);
        for j in 0..TAIL_FADE {
            let g = 0.5 + 0.5 * (std::f32::consts::PI * (j + 1) as f32 / TAIL_FADE as f32).cos();
            dst[EQUALIZED_IR_LENGTH - TAIL_FADE + j] *= g;
        }
    }

    db.impulse_responses = out;
    db.ir_length = EQUALIZED_IR_LENGTH;
    db.sample_rate = target_fs;
    db.data_sample_rate = Some(target_fs);
}

/// 출력 샘플 하나를 만드는 리샘플링 커널: (입력 시작 인덱스, 가중치들).
type ResampleKernel = (usize, Vec<f32>);

/// 윈도우드 싱크 리샘플러 커널(±32샘플, 블랙맨 창). 다운샘플이면 대역을 먼저 제한한다.
/// 샘플레이트가 같으면 항등 커널을 돌려준다.
fn resample_kernels(in_len: usize, fs_in: f32, fs_out: f32) -> Vec<ResampleKernel> {
    if fs_in <= 0.0 || (fs_in - fs_out).abs() < 0.5 {
        return (0..in_len).map(|n| (n, vec![1.0])).collect();
    }
    const HALF: i32 = 32;
    let ratio = fs_out / fs_in;
    let fc = ratio.min(1.0);
    let out_len = (in_len as f32 * ratio).ceil() as usize;
    (0..out_len)
        .map(|n| {
            let t = n as f32 / ratio;
            let center = t.floor() as i32;
            let first = (center - HALF + 1).max(0);
            let last = (center + HALF).min(in_len as i32 - 1);
            let weights = (first..=last)
                .map(|k| {
                    let d = t - k as f32;
                    let u = d / HALF as f32;
                    if u.abs() >= 1.0 {
                        return 0.0;
                    }
                    let w = 0.42
                        + 0.5 * (std::f32::consts::PI * u).cos()
                        + 0.08 * (2.0 * std::f32::consts::PI * u).cos();
                    let arg = std::f32::consts::PI * fc * d;
                    let sinc = if arg.abs() < 1e-6 { 1.0 } else { arg.sin() / arg };
                    fc * sinc * w
                })
                .collect();
            (first.max(0) as usize, weights)
        })
        .collect()
}

fn apply_kernels(x: &[f32], kernels: &[ResampleKernel]) -> Vec<f32> {
    kernels
        .iter()
        .map(|(start, w)| {
            w.iter()
                .enumerate()
                .map(|(j, g)| x.get(start + j).copied().unwrap_or(0.0) * g)
                .sum()
        })
        .collect()
}

/// 도달 시각(샘플, 소수점 포함): 최대값의 [ONSET_THRESHOLD]를 처음 넘는 지점.
fn onset(ir: &[f32]) -> f32 {
    let max = ir.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    if max <= 0.0 {
        return 0.0;
    }
    let th = ONSET_THRESHOLD * max;
    match ir.iter().position(|v| v.abs() >= th) {
        Some(0) | None => 0.0,
        Some(i) => {
            let (a, b) = (ir[i - 1].abs(), ir[i].abs());
            let frac = if b > a { (th - a) / (b - a) } else { 0.0 };
            (i - 1) as f32 + frac.clamp(0.0, 1.0)
        }
    }
}

/// 파워 스펙트럼을 1/`fraction` 옥타브로 평활해 크기로 돌려준다.
fn smooth_fractional_octave(power: &[f64], bin_hz: f32, fraction: f32) -> Vec<f32> {
    let n = power.len();
    let mut prefix = vec![0.0f64; n + 1];
    for (i, p) in power.iter().enumerate() {
        prefix[i + 1] = prefix[i] + p;
    }
    let spread = 2.0f32.powf(1.0 / (2.0 * fraction));
    (0..n)
        .map(|k| {
            let f = (k as f32 * bin_hz).max(bin_hz);
            let lo = ((f / spread / bin_hz).floor() as usize).min(n - 1);
            let hi = ((f * spread / bin_hz).ceil() as usize).clamp(lo + 1, n);
            (((prefix[hi] - prefix[lo]) / (hi - lo) as f64).sqrt()) as f32
        })
        .collect()
}

/// 크기 스펙트럼(0..=N/2)으로 최소위상 IR을 만들고 `delay`샘플만큼 늦춘다(켑스트럼 방법).
fn min_phase_with_delay(
    mag: &[f32],
    delay: f32,
    r2c: &dyn realfft::RealToComplex<f32>,
    c2r: &dyn realfft::ComplexToReal<f32>,
) -> Vec<f32> {
    let n = FFT_SIZE;
    let half = n / 2;
    // 실수 켑스트럼
    let mut log_mag: Vec<Complex<f32>> =
        mag.iter().map(|&m| Complex::new(m.max(1e-5).ln(), 0.0)).collect();
    let mut cep = vec![0.0f32; n];
    let _ = c2r.process(&mut log_mag, &mut cep);
    for v in cep.iter_mut() {
        *v /= n as f32;
    }
    // 인과 쪽으로 접는다 -> 최소위상
    for v in cep.iter_mut().take(half).skip(1) {
        *v *= 2.0;
    }
    for v in cep.iter_mut().skip(half + 1) {
        *v = 0.0;
    }
    let mut spec = r2c.make_output_vec();
    let _ = r2c.process(&mut cep, &mut spec);
    for (k, c) in spec.iter_mut().enumerate() {
        let amp = c.re.exp();
        let phase = c.im - std::f32::consts::TAU * k as f32 * delay / n as f32;
        *c = Complex::new(amp * phase.cos(), amp * phase.sin());
    }
    spec[0].im = 0.0;
    spec[half].im = 0.0;
    let mut h = vec![0.0f32; n];
    let _ = c2r.process(&mut spec, &mut h);
    for v in h.iter_mut() {
        *v /= n as f32;
    }
    h
}
