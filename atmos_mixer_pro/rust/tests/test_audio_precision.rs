use rust_lib_atmos_mixer_pro::audio::spatial::Spatializer3D;
use rust_lib_atmos_mixer_pro::audio::reverb::VirtualRoomReverb;

#[test]
fn test_audio_precision_sine_wave() {
    let sample_rate = 48000.0;
    
    // 1kHz Sine Wave Generation
    let frequency = 1000.0;
    let mut sine_wave = vec![0.0; 48000]; // 1 second
    for i in 0..sine_wave.len() {
        sine_wave[i] = (2.0 * std::f32::consts::PI * frequency * (i as f32) / sample_rate).sin();
    }
    
    // Test 1: Silence
    let mut reverb = VirtualRoomReverb::new(sample_rate);
    let (out_l, out_r) = reverb.process_stereo(0.0, 0.0);
    assert!(out_l.abs() < 0.001, "Silence should output silence (error: {})", out_l.abs());
    assert!(out_r.abs() < 0.001, "Silence should output silence (error: {})", out_r.abs());
    
    // Test 2: Spatializer DBAP Precision
    let positions = vec![
        (1.0, 0.0, 0.0), // Right
        (-1.0, 0.0, 0.0), // Left
    ];
    let mut spatializer = Spatializer3D::new(positions, sample_rate);
    let mut out_buffer = vec![0.0_f32; 2];
    
    // Process sine wave at center (0, 0, 0)
    spatializer.process_sample(sine_wave[100], (0.0, 0.0, 0.0), 0.0, &mut out_buffer);
    
    // Distance to both speakers is 1.0. Gain should be equal.
    let diff = (out_buffer[0].abs() - out_buffer[1].abs()).abs();
    assert!(diff < 0.001, "Precision error in DBAP Center panning: diff {}", diff);
    
    // Ensure no NaN
    assert!(!out_buffer[0].is_nan() && !out_buffer[1].is_nan());
}
