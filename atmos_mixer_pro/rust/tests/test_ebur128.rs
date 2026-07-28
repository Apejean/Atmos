use ebur128::{EbuR128, Mode};

#[test]
fn test_lufs_digital_silence() {
    let sample_rate = 48000;
    let channels = 2;
    let mut meter = EbuR128::new(channels, sample_rate, Mode::M | Mode::S | Mode::I | Mode::TRUE_PEAK).unwrap();
    
    // 1 second of digital silence
    let silence = vec![0.0f32; (sample_rate * channels) as usize];
    meter.add_frames_f32(&silence).unwrap();
    
    let m = meter.loudness_momentary().unwrap();
    assert!(m < -100.0, "Silence should be less than -100 LUFS, got {}", m);
}

#[test]
fn test_lufs_1khz_sine() {
    let sample_rate = 48000;
    let channels = 2;
    let mut meter = EbuR128::new(channels, sample_rate, Mode::M | Mode::S | Mode::I | Mode::TRUE_PEAK).unwrap();
    
    let mut sine = vec![0.0f32; (sample_rate * channels) as usize];
    let freq = 1000.0;
    for i in 0..sample_rate {
        let sample = (2.0 * std::f32::consts::PI * freq * (i as f32) / (sample_rate as f32)).sin() * 0.5; // -6dBFS
        sine[(i * channels) as usize] = sample;
        sine[(i * channels + 1) as usize] = sample;
    }
    
    meter.add_frames_f32(&sine).unwrap();
    
    let m = meter.loudness_momentary().unwrap();
    // A full scale sine wave is approx -3.01 LUFS. At 0.5 (-6dBFS) it should be around -9 LUFS
    assert!(m > -12.0 && m < -6.0, "1kHz sine at 0.5 should be approx -9 LUFS, got {}", m);
}
