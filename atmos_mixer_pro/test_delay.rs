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
    for i in 0..10 {
        let input = i as f32;
        delay_buffer[delay_write_idx] = input;
        
        let current_delay_ms = 0.05_f32; // 2.4 samples
        let fs = 48000.0_f32;
        let delay_samples = (current_delay_ms / 1000.0 * fs).clamp(0.0, 48000.0 - 4.0);
        let delay_int = delay_samples.floor() as usize; // 2
        let delay_frac = delay_samples - delay_int as f32; // 0.4
        
        // Let's print out what indices we read
        let read_idx_x0 = (delay_write_idx + 48000 - delay_int + 1) % 48000;
        let read_idx_x1 = (delay_write_idx + 48000 - delay_int) % 48000;
        let read_idx_x2 = (delay_write_idx + 48000 - delay_int - 1) % 48000;
        let read_idx_x3 = (delay_write_idx + 48000 - delay_int - 2) % 48000;
        
        println!("write_idx: {} -> read_indices: x0:{} x1:{} x2:{} x3:{}", delay_write_idx, read_idx_x0, read_idx_x1, read_idx_x2, read_idx_x3);
        delay_write_idx = (delay_write_idx + 1) % 48000;
    }
}
