use crate::common::config::{EqBand, EqType};
use std::f32::consts::PI;

/// Computes the magnitude response (in dB) of a given EqBand at a specific frequency `f`.
/// This matches the Cytomic ZDF SVF implementation found in svf.rs.
pub fn calculate_svf_magnitude_db(band: &EqBand, fs: f32, f: f32) -> f32 {
    if !band.enabled || f <= 0.0 || f >= fs / 2.0 {
        return 0.0;
    }

    let freq = band.freq.clamp(10.0, fs * 0.49);
    let q = band.q_factor.max(0.01);
    let gain_db = band.gain;

    let g = (PI * freq / fs).tan();
    let k = 1.0 / q;

    // Calculate coefficients based on filter type (same as svf.rs)
    let (a1, m0, m1, m2) = match band.filter_type {
        EqType::Bell => {
            let a = 10.0f32.powf(gain_db / 40.0);
            let a1 = 1.0 / (1.0 + g * (k / a) + g * g);
            (a1, 1.0, k * (a - 1.0 / a), 0.0)
        }
        EqType::LowShelf => {
            let a = 10.0f32.powf(gain_db / 40.0);
            let a1 = 1.0 / (1.0 + g * (k / a) + g * g);
            (a1, 1.0, k * (a - 1.0 / a), a * a - 1.0)
        }
        EqType::HighShelf => {
            let a = 10.0f32.powf(gain_db / 40.0);
            let a1 = 1.0 / (1.0 + g * (k * a) + g * g);
            (a1, a * a, k * (1.0 / a - a) * a * a, 1.0 - a * a)
        }
        EqType::LowCut => { // HighPass
            let a1 = 1.0 / (1.0 + g * k + g * g);
            (a1, 1.0, -k, -1.0)
        }
        EqType::HighCut => { // LowPass
            let a1 = 1.0 / (1.0 + g * k + g * g);
            (a1, 0.0, 0.0, 1.0)
        }
        EqType::Notch => {
            let a1 = 1.0 / (1.0 + g * k + g * g);
            (a1, 1.0, -k, 0.0)
        }
    };

    let a2 = g * a1;
    // a3 = g * a2 is needed in standard SVF difference equations, but we only use a1, a2 for analog mapping
    // let a3 = g * a2;
    
    // --- Pre-warped Analog Evaluation ---
    // The ZDF SVF perfectly maps the analog SVF via Bilinear Transform.
    // Analog frequency: s = j * wa
    let wa = (PI * f / fs).tan(); // Pre-warped frequency
    
    // Analog SVF transfer function components (normalized to wc = g)
    // s_n = s / wc = j * wa / g = j * w_n
    let w_n = wa / g;
    
    // Denominator: D(s) = s_n^2 + k*s_n + 1 = (1 - w_n^2) + j*(k*w_n)
    let re_d = 1.0 - w_n * w_n;
    let im_d = k * w_n;
    let mag_d_sq = re_d * re_d + im_d * im_d;
    
    // Numerators N(s) depending on filter type:
    let (re_n, im_n) = match band.filter_type {
        EqType::Bell => {
            let a = 10.0f32.powf(gain_db / 40.0);
            let k_num = k * a;
            // N(s) = s_n^2 + k_num*s_n + 1 = (1 - w_n^2) + j*(k_num*w_n)
            (1.0 - w_n * w_n, k_num * w_n)
        }
        EqType::LowShelf => {
            let a = 10.0f32.powf(gain_db / 40.0);
            let k_num = k / a;
            // N(s) = s_n^2 + k_num*s_n + a^2 = (a^2 - w_n^2) + j*(k_num*w_n)
            (a * a - w_n * w_n, k_num * w_n)
        }
        EqType::HighShelf => {
            let a = 10.0f32.powf(gain_db / 40.0);
            let k_num = k * a;
            // N(s) = a^2 * s_n^2 + k_num*s_n + 1 = (1 - a^2*w_n^2) + j*(k_num*w_n)
            (1.0 - a * a * w_n * w_n, k_num * w_n)
        }
        EqType::LowCut => { // HighPass
            // N(s) = s_n^2 = -w_n^2
            (-w_n * w_n, 0.0)
        }
        EqType::HighCut => { // LowPass
            // N(s) = 1
            (1.0, 0.0)
        }
        EqType::Notch => {
            // N(s) = s_n^2 + 1 = 1 - w_n^2
            (1.0 - w_n * w_n, 0.0)
        }
    };
    
    let mag_n_sq = re_n * re_n + im_n * im_n;
    
    // Magnitude squared: |H(j w_a)|^2 = |N|^2 / |D|^2
    let mag_sq = if mag_d_sq > 1e-15 { mag_n_sq / mag_d_sq } else { 1.0 };
    
    let mag = mag_sq.sqrt().max(1e-5);
    20.0 * mag.log10()
}

/// Generates a logarithmic sequence of frequencies between `f_min` and `f_max`.
pub fn generate_log_frequencies(f_min: f32, f_max: f32, points: usize) -> Vec<f32> {
    let mut freqs = Vec::with_capacity(points);
    if points == 0 || f_min <= 0.0 || f_max <= f_min {
        return freqs;
    }
    let log_min = f_min.ln();
    let log_max = f_max.ln();
    let step = (log_max - log_min) / (points as f32 - 1.0);
    
    for i in 0..points {
        let f = (log_min + i as f32 * step).exp();
        freqs.push(f);
    }
    freqs
}

/// Computes the total combined EQ curve (dB) for a list of bands over a given array of frequencies.
pub fn calculate_total_eq_curve(bands: &[EqBand], fs: f32, freqs: &[f32]) -> Vec<f32> {
    let mut curve = vec![0.0; freqs.len()];
    for band in bands {
        if !band.enabled { continue; }
        for (i, &f) in freqs.iter().enumerate() {
            curve[i] += calculate_svf_magnitude_db(band, fs, f);
        }
    }
    curve
}
