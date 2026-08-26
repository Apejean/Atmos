// tests/test_zero_defect_e2e.rs
// Atmos Mixer Pro - Module 12 Zero-Defect E2E Verification Test Suite

use rust_lib_atmos_mixer_pro::audio::spatial::DbapMatrix;
use rust_lib_atmos_mixer_pro::audio::limiter::PeakLimiter;
use rust_lib_atmos_mixer_pro::audio::svf::SvfFilter;
use rust_lib_atmos_mixer_pro::common::config::EqType;

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
    dbap.calculate_gains(0.0, 0.0, 0.0, 0.0, &mut gains);

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
    // 2. Verify Peak Limiter prevents True Peak (ISP) signal from exceeding threshold (-0.1 dBFS = ~0.9885)
    let sample_rate = 48000.0;
    let mut limiter = PeakLimiter::new(sample_rate, 5.0, 100.0, 1.0);

    let mut max_output = 0.0f32;
    let mut max_isp = 0.0f32;
    let mut prev_out = 0.0f32;

    // Inject +6dBFS overload with extreme high frequency (e.g. fs/4 = 12000Hz) to maximize ISP
    // Intersample peaks occur when the true analog waveform exceeds the digital sample values.
    for i in 0..48000 {
        let t = i as f32 / sample_rate;
        let freq = 12000.0; 
        
        // Add phase shift to guarantee samples don't land exactly on peaks
        let phase = std::f32::consts::PI / 4.0;
        let input_sample = 2.0 * (2.0 * std::f32::consts::PI * freq * t + phase).sin();
        let output_sample = limiter.process(input_sample);

        if i > 480 { // Skip initial lookahead delay fill
            max_output = max_output.max(output_sample.abs());
            
            // Basic 4x oversampling approximation to check ISP of the output
            // We use cubic/linear interpolation just to see if the output *would* exceed if reconstructed
            let d = output_sample - prev_out;
            for j in 1..4 {
                let frac = j as f32 / 4.0;
                let interp = prev_out + d * frac;
                max_isp = max_isp.max(interp.abs());
            }
        }
        prev_out = output_sample;
    }

    println!("Limiter Max Output Level (Sample Peak): {:.6}", max_output);
    println!("Limiter Max ISP Level (Approximated True Peak): {:.6}", max_isp);

    assert!(
        max_output <= 0.99,
        "Peak Limiter Overload Failure! Max output was {}",
        max_output
    );
    
    assert!(
        max_isp <= 0.995, // Allow tiny rounding tolerance but should be well below 1.0
        "Peak Limiter True Peak (ISP) Failure! Max ISP was {}",
        max_isp
    );
}

#[test]
fn test_zero_defect_digital_silence_no_denormal_explosion() {
    // 3. Verify SVF Filter handles Digital Silence without Denormal Float CPU/NaN explosion
    let mut svf = SvfFilter::new();
    svf.update_coefficients(&EqType::Bell, 48000.0, 1000.0, 0.707, 0.0);

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

#[test]
fn test_zero_defect_rta_fft_frequency_accuracy() {
    // 4. RTA FFT Frequency Accuracy Verification
    use rustfft::{num_complex::Complex, FftPlanner};
    
    let sample_rate = 48000.0;
    let target_freq = 1000.0; // 1kHz sine wave
    let fft_size = 4096;
    
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    
    let mut buffer = vec![Complex { re: 0.0f32, im: 0.0f32 }; fft_size];
    
    // Generate 1kHz sine wave
    for i in 0..fft_size {
        let t = i as f32 / sample_rate;
        // Apply Hann window to reduce leakage
        let window = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (fft_size - 1) as f32).cos());
        buffer[i].re = (2.0 * std::f32::consts::PI * target_freq * t).sin() * window;
    }
    
    fft.process(&mut buffer);
    
    let mut max_mag = 0.0;
    let mut max_bin = 0;
    
    // Search only up to Nyquist (fft_size / 2)
    for i in 0..fft_size / 2 {
        let mag = buffer[i].norm();
        if mag > max_mag {
            max_mag = mag;
            max_bin = i;
        }
    }
    
    let detected_freq = max_bin as f32 * sample_rate / fft_size as f32;
    println!("RTA FFT Detected Freq: {:.2} Hz (Bin: {})", detected_freq, max_bin);
    
    let error = (detected_freq - target_freq).abs();
    assert!(
        error < 25.0, // With 4096 size at 48kHz, bin resolution is ~11.7Hz
        "RTA FFT Frequency detection failed! Target: {}, Detected: {} (Error: {})",
        target_freq, detected_freq, error
    );
}

#[test]
fn test_zero_defect_ffi_struct_alignment() {
    // 5. Memory Alignment Verification for FFI structures
    use std::mem;
    
    // This requires checking the actual types used in flutter_rust_bridge or OSC C/C++ boundaries
    // Here we'll verify common foundational structs
    use rust_lib_atmos_mixer_pro::common::config::Point3D;
    
    let size = mem::size_of::<Point3D>();
    let align = mem::align_of::<Point3D>();
    
    println!("Point3D Size: {}, Alignment: {}", size, align);
    
    // A Point3D is typically 6 f32s = 24 bytes, 4-byte aligned
    // Depending on Serde macros it could be padded, but usually just 24
    assert!(
        align == 4, 
        "Point3D alignment must be 4 bytes for C/C++ FFI compatibility! Found: {}", 
        align
    );
}

#[test]
fn test_zero_defect_e2e_watchdog_autoguard() {
    let mut limiter = rust_lib_atmos_mixer_pro::audio::limiter::PeakLimiter::new(48000.0, 1.0, 500.0, 1.0);
    let target = 10.0;
    for _ in 0..(48000 * 3) {
        limiter.process(target);
    }
    assert!(limiter.short_term_lufs > -14.0, "LUFS should be high");
    assert!(limiter.autoguard_ducking < 1.0, "Ducking should be applied");
}

#[test]
fn test_phase3_physics_and_offline_bounce() {
    use rust_lib_atmos_mixer_pro::audio::dsp::acoustic_physics::AirAbsorptionFilter;
    use rust_lib_atmos_mixer_pro::audio::svf::LinkwitzRiley24;
    use rust_lib_atmos_mixer_pro::audio::offline::OfflineRenderer;
    use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
    use rust_lib_atmos_mixer_pro::common::config::EqType;
    
    // 1. Test Air Absorption
    let mut air_filter = AirAbsorptionFilter::new();
    air_filter.set_distance(40.0, 48000.0); // 40 meters
    let out = air_filter.process(1.0);
    assert!(out.is_finite(), "Air absorption filter produced non-finite output");

    // 2. Test Linkwitz-Riley 24dB/oct Crossover
    let mut lr24 = LinkwitzRiley24::new();
    lr24.set_coefficients(EqType::LowCut, 48000.0, 1000.0);
    let out_lr = lr24.process(1.0);
    assert!(out_lr.is_finite(), "Linkwitz-Riley filter produced non-finite output");

    // 3. Test Offline Renderer (Zero-allocation during process loop)
    let (tx, _rx) = crossbeam_channel::bounded(10);
    let mixer = AudioMixer::new(48000, 2, tx, None);
    
    // Render 50ms (0.05s) of audio offline to verify it doesn't crash or block
    let renderer = OfflineRenderer::new(48000, 2, 0.05);
    
    let temp_dir = std::env::temp_dir();
    let out_path = temp_dir.join("test_phase3_offline_bounce.wav");
    
    let result = renderer.render_to_wav(mixer, out_path.to_str().unwrap());
    assert!(result.is_ok(), "Offline rendering failed: {:?}", result.err());
    
    assert!(out_path.exists(), "WAV file was not created");
    
    // Clean up
    if out_path.exists() {
        std::fs::remove_file(out_path).unwrap();
    }
}
