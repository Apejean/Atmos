use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use std::time::Instant;

fn main() {
    let (gc_tx, _gc_rx) = crossbeam_channel::bounded(4096);
    let _default_config = rust_lib_atmos_mixer_pro::common::config::AppConfig::default();
    let mut mixer = AudioMixer::new(48000, gc_tx);
    
    // Enable 24 channels
    for ch in 0..24 {
        rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE.enabled_channels[ch].store(true, std::sync::atomic::Ordering::Relaxed);
        mixer.channel_dsp[ch].target_delay_ms = 100.0;
    }
    
    let mut output = vec![0.0; 256 * 24]; // 256 samples, 24 channels
    
    // Warm up
    mixer.process(&mut output, 24);
    
    let iterations = 1000;
    let start = Instant::now();
    for _ in 0..iterations {
        mixer.process(&mut output, 24);
    }
    let duration = start.elapsed();
    let per_call = duration.as_micros() as f64 / iterations as f64;
    
    println!("Processed {} buffers in {:?}", iterations, duration);
    println!("Average time per process (256 samples * 24ch): {:.2} us", per_call);
    if per_call < 5000.0 {
        println!("BENCHMARK PASSED (well under 5000 us = 5 ms)");
    } else {
        println!("BENCHMARK FAILED (took too long)");
    }
}
