fn main() {
    let speed_of_sound = 343.0; // m/s
    let target_dist = 10.0;
    
    // Max human run speed = 10m/s
    // 10m / 10m/s = 1s = 48000 samples.
    // If distance changes by 10m over 1 second:
    let delta_dist_per_sample = 10.0 / 48000.0; // meters per sample
    
    // delay = dist / 343
    // delta_delay_sec = delta_dist / 343 = (10/48000) / 343
    let delta_delay_sec = delta_dist_per_sample / speed_of_sound;
    
    // delta_delay_samples = delta_delay_sec * 48000
    let delta_delay_samples = delta_delay_sec * 48000.0;
    
    println!("Physical doppler shift samples per sample: {:.6}", delta_delay_samples);
    
    // Now compare with current logic which interpolates instantly if moved fast:
    // we use 0.005 smoothing
    let start_delay_ms = 0.0;
    let target_delay_ms = (10.0 / 343.0) * 1000.0;
    
    let diff = target_delay_ms - start_delay_ms;
    let step1 = diff * 0.005; // ms
    let step1_samples = step1 / 1000.0 * 48000.0;
    
    println!("First step of 0.005 smoothing shift samples per sample: {:.6}", step1_samples);
}
