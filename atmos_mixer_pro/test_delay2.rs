fn main() {
    let mut current_delay_ms = 0.5_f32; 
    let mut current_distance = 0.0;
    
    let target_dist = 1.0;
    
    for _ in 0..100 {
        let diff_dist = target_dist - current_distance;
        if diff_dist.abs() > 0.01 {
            current_distance += diff_dist * 0.005;
        } else {
            current_distance = target_dist;
        }
        println!("{}", current_distance);
    }
}
