fn main() {
    let sample_rate = 48000.0;
    let delay_samples = ((sample_rate * 0.005) as usize).max(1);
    println!("delay_samples: {}", delay_samples);
}
