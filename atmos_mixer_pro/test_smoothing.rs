fn main() {
    let mut current_delay_ms: f32 = 0.0;
    let target_delay_ms: f32 = 10.0; // 10ms delay
    
    // We call this process() 48000 times a second
    // let's see how long it takes to reach target
    let mut samples = 0;
    let mut ms_passed = 0.0;
    loop {
        let diff = target_delay_ms - current_delay_ms;
        if diff.abs() > 0.001 {
            current_delay_ms += diff * 0.005;
        } else {
            current_delay_ms = target_delay_ms;
            break;
        }
        samples += 1;
        if samples % 48 == 0 {
            ms_passed += 1.0;
        }
    }
    println!("Reached target in {} samples ({} ms)", samples, ms_passed);
}
