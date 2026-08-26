pub fn interpolate_hermite(x0: f32, x1: f32, x2: f32, x3: f32, t: f32) -> f32 {
    let diff = x1 - x2;
    let c1 = x2 - x0;
    let c3 = x3 - x0 + 3.0 * diff;
    let c2 = -(2.0 * diff + c1 + c3);
    0.5 * ((c3 * t + c2) * t + c1) * t + x1
}
fn main() {
    let mut delay_buffer = vec![0.0; 48000];
    let mut delay_write_idx = 0;
    
    let mut current_delay_ms: f32 = 0.0;
    let target_delay_ms: f32 = 1.0; 
    let fs = 48000.0;
    
    for i in 0..1000 {
        let diff = target_delay_ms - current_delay_ms;
        if diff.abs() > 0.001 {
            current_delay_ms += diff * 0.005; // Smoothing factor
        } else {
            current_delay_ms = target_delay_ms;
        }
        
        let input = (i as f32 * 0.01).sin();
        delay_buffer[delay_write_idx] = input;
        
        let delay_samples = (current_delay_ms / 1000.0 * fs).clamp(0.0, 48000.0 - 4.0);
        let delay_int = delay_samples.floor() as usize;
        let delay_frac = delay_samples - delay_int as f32;
        
        // This is where Doppler shift happens! If delay_samples is increasing by X samples per sample...
        if i % 100 == 0 {
            println!("t={} current_delay: {:.4}ms (samples: {:.2})", i, current_delay_ms, delay_samples);
        }
        
        delay_write_idx = (delay_write_idx + 1) % 48000;
    }
}
