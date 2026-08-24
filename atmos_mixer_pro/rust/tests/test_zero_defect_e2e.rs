// tests/test_zero_defect_e2e.rs
// Atmos Mixer Pro - Module 12 Zero-Defect E2E Verification Test Suite

use atmos_mixer_pro_lib::audio::spatial::DbapMatrix;
use atmos_mixer_pro_lib::audio::limiter::PeakLimiter;
use atmos_mixer_pro_lib::audio::svf::{SvfFilter, FilterType};

#[test]
fn test_zero_defect_dbap_panning_power_normalization() {
    // 1. Verify DBAP 3D Panning Power Normalization (sum of gain squares == 1.0)
    let speaker_positions = vec![
        (-2.0, 2.0, 0.0),  // Left
        (2.0, 2.0, 0.0),   // Right
        (0.0, 2.0, 2.0),   // Height
        (0.0, -2.0, 0.0),  // Rear
    ];

    let dbap = DbapMatrix::new(speaker_positions);
    let mut gains = vec![0.0f32; 4];

    // Source at center (0, 0, 0)
    dbap.calculate_gains(0.0, 0.0, 0.0, &mut gains);

    let power_sum: f32 = gains.iter().map(|&g| g * g).sum();
    println!("DBAP Center Power Sum: {:.6}", power_sum);

    assert!(
        (power_sum - 1.0).abs() < 1e-4,
        "DBAP Power Normalization failed! Power sum = {}",
        power_sum
    );
}

#[test]
fn test_zero_defect_peak_limiter_true_peak_margin() {
    // 2. Verify Peak Limiter prevents signal from exceeding threshold (-0.1 dBFS = ~0.9885)
    let sample_rate = 48000.0;
    let mut limiter = PeakLimiter::new(sample_rate, 5.0, 100.0, 1.0);

    let mut max_output = 0.0f32;

    // Inject massive +6dBFS overload sine wave (amplitude = 2.0)
    for i in 0..4800 {
        let t = i as f32 / sample_rate;
        let input_sample = 2.0 * (2.0 * std::f32::consts::PI * 1000.0 * t).sin();
        let output_sample = limiter.process(input_sample);

        if i > 240 { // Skip initial lookahead delay fill
            max_output = max_output.max(output_sample.abs());
        }
    }

    println!("Limiter Max Output Level: {:.6}", max_output);

    assert!(
        max_output <= 0.99,
        "Peak Limiter Overload Failure! Max output was {}",
        max_output
    );
}

#[test]
fn test_zero_defect_digital_silence_no_denormal_explosion() {
    // 3. Verify SVF Filter handles Digital Silence without Denormal Float CPU/NaN explosion
    let mut svf = SvfFilter::new();
    svf.configure(FilterType::Bell, 1000.0, 0.0, 1.0, 48000.0);

    // Process 100,000 silent samples
    for _ in 0..100000 {
        let out = svf.process(0.0);
        assert!(
            !out.is_nan() && !out.is_infinite(),
            "SVF Filter generated NaN/Inf on silence!"
        );
        assert!(
            out.abs() < 1e-6,
            "SVF Filter leaking non-zero noise on silence!"
        );
    }
}
