fn main() {
    let mut current_delay_ms: f32 = 0.0;
    let target_delay_ms: f32 = 1.0; 
    let fs = 48000.0;
    
    let mut last_delay_samples = 0.0;
    
    for _ in 0..100 {
        let diff = target_delay_ms - current_delay_ms;
        if diff.abs() > 0.001 {
            current_delay_ms += diff * 0.00005; // 100x slower
        } else {
            current_delay_ms = target_delay_ms;
        }
        
        let delay_samples = (current_delay_ms / 1000.0 * fs).clamp(0.0, 48000.0 - 4.0);
        let delta_samples = delay_samples - last_delay_samples;
        
        println!("Delta samples per step: {:.4}", delta_samples);
        last_delay_samples = delay_samples;
    }
}
