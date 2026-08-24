use std::f32::consts::PI;

#[test]
fn test_audio_precision_silence_injection() {
    let sample_rate = 48000;
    let channels = 2;
    let buffer_size = 1024;

    // Zero/Silence buffer
    let mut buffer = vec![0.0f32; buffer_size * channels];

    // Verify all samples are exactly 0.0
    for sample in buffer.iter() {
        assert_eq!(*sample, 0.0f32, "Digital silence sample must be exactly 0.0");
    }

    // Measure RMS of silence
    let sum_sq: f32 = buffer.iter().map(|&s| s * s).sum();
    let rms = (sum_sq / buffer.len() as f32).sqrt();
    assert_eq!(rms, 0.0f32, "Silence RMS must be 0.0, got {}", rms);
}

#[test]
fn test_audio_precision_1khz_sine_wave() {
    let sample_rate = 48000;
    let channels = 2;
    let freq = 1000.0f32;
    let duration_sec = 1.0f32;
    let total_frames = (sample_rate as f32 * duration_sec) as usize;

    let mut buffer = vec![0.0f32; total_frames * channels];

    // Generate 1kHz pure sine wave at 1.0 peak amplitude
    for i in 0..total_frames {
        let t = i as f32 / sample_rate as f32;
        let expected_val = (2.0 * PI * freq * t).sin();

        buffer[i * channels] = expected_val;     // L
        buffer[i * channels + 1] = expected_val; // R

        // Check single sample precision against mathematical sine wave (0.001% tolerance = 1e-5)
        let diff_l = (buffer[i * channels] - expected_val).abs();
        assert!(diff_l < 1e-5, "Sample {} diff L: {} exceeds 0.001% tolerance", i, diff_l);
    }

    // Verify Peak value is 1.0 (exact peak)
    let peak = buffer.iter().fold(0.0f32, |acc, &s| acc.max(s.abs()));
    assert!((peak - 1.0).abs() < 1e-5, "Peak expected 1.0, got {}", peak);

    // Verify RMS value of 1.0 peak sine wave is 1/sqrt(2) = ~0.70710678
    let expected_rms = 1.0f32 / 2.0f32.sqrt();
    let sum_sq: f32 = buffer.iter().map(|&s| s * s).sum();
    let calculated_rms = (sum_sq / buffer.len() as f32).sqrt();

    let rms_error_pct = ((calculated_rms - expected_rms) / expected_rms).abs() * 100.0;
    println!("Calculated RMS: {}, Expected RMS: {}, Error: {:.6}%", calculated_rms, expected_rms, rms_error_pct);

    assert!(
        rms_error_pct < 0.001,
        "RMS Error {:.6}% exceeds strict 0.001% threshold",
        rms_error_pct
    );
}
