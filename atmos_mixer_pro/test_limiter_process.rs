use std::f32::consts::PI;

pub struct PeakLimiter {
    fast_attack_coef: f32,
    fast_release_coef: f32,
    slow_attack_coef: f32,
    slow_release_coef: f32,
    fast_envelope: f32,
    slow_envelope: f32,
    threshold: f32,
    knee_width: f32,
    delay_buffer: Vec<f32>,
    delay_index: usize,
    hold_counter: usize,
    hold_samples: usize,
    history: [f32; 3],
    pub autoguard_ducking: f32,
}

impl PeakLimiter {
    pub fn new(sample_rate: f32, attack_ms: f32, release_ms: f32, threshold: f32) -> Self {
        let actual_threshold = threshold * 0.97;
        let delay_samples = ((sample_rate * 0.005) as usize).max(1);
        let delay_buffer = vec![0.0; delay_samples];
        let detector_rate = sample_rate * 4.0;
        let fast_attack_sec = 0.0001; 
        let fast_release_sec = 0.050; 
        let slow_attack_sec = attack_ms / 1000.0;
        let slow_release_sec = release_ms / 1000.0; 
        let fast_attack_coef = if fast_attack_sec > 0.0 { (-1.0 / (fast_attack_sec * detector_rate)).exp() } else { 0.0 };
        let fast_release_coef = if fast_release_sec > 0.0 { (-1.0 / (fast_release_sec * detector_rate)).exp() } else { 0.0 };
        let slow_attack_coef = if slow_attack_sec > 0.0 { (-1.0 / (slow_attack_sec * detector_rate)).exp() } else { 0.0 };
        let slow_release_coef = if slow_release_sec > 0.0 { (-1.0 / (slow_release_sec * detector_rate)).exp() } else { 0.0 };

        Self {
            fast_attack_coef, fast_release_coef, slow_attack_coef, slow_release_coef,
            fast_envelope: 0.0, slow_envelope: 0.0,
            threshold: actual_threshold, knee_width: 0.005,
            delay_buffer, delay_index: 0,
            hold_counter: 0, hold_samples: delay_samples * 4,
            history: [0.0; 3], autoguard_ducking: 1.0,
        }
    }

    pub fn process(&mut self, sample: f32) -> f32 {
        let mut max_abs = 0.0_f32;
        let x0 = self.history[0]; let x1 = self.history[1]; let x2 = self.history[2]; let x3 = sample;
        let c0 = x1; let c1 = 0.5 * (x2 - x0); let c2 = x0 - 2.5 * x1 + 2.0 * x2 - 0.5 * x3; let c3 = 0.5 * (x3 - x0) + 1.5 * (x1 - x2);

        for i in 1..=4 {
            let t = i as f32 / 4.0;
            let interp = ((c3 * t + c2) * t + c1) * t + c0;
            let abs_val = interp.abs();
            if abs_val > max_abs { max_abs = abs_val; }
            
            if abs_val > self.fast_envelope {
                self.fast_envelope = self.fast_attack_coef * (self.fast_envelope - abs_val) + abs_val;
                self.hold_counter = self.hold_samples;
            } else {
                if self.hold_counter > 0 { self.hold_counter -= 1; } 
                else { self.fast_envelope = self.fast_release_coef * (self.fast_envelope - abs_val) + abs_val; }
            }
            if abs_val > self.slow_envelope { self.slow_envelope = self.slow_attack_coef * (self.slow_envelope - abs_val) + abs_val; } 
            else { self.slow_envelope = self.slow_release_coef * (self.slow_envelope - abs_val) + abs_val; }
        }
        
        self.history[0] = self.history[1]; self.history[1] = self.history[2]; self.history[2] = sample;
        let envelope = self.fast_envelope.max(self.slow_envelope);
        
        let mut gain = 1.0;
        let knee_lower = self.threshold - self.knee_width / 2.0;
        let knee_upper = self.threshold + self.knee_width / 2.0;
        if envelope > knee_lower {
            if envelope < knee_upper {
                let x = envelope - knee_lower;
                let target = knee_lower + (x * x) / (2.0 * self.knee_width);
                gain = target / envelope;
            } else { gain = self.threshold / envelope; }
        }
        
        let delayed_sample = self.delay_buffer[self.delay_index];
        self.delay_buffer[self.delay_index] = sample;
        self.delay_index = (self.delay_index + 1) % self.delay_buffer.len();
        
        let out = delayed_sample * gain;
        out
    }
}

fn main() {
    let mut limiter = PeakLimiter::new(48000.0, 1.0, 300.0, 0.9);
    let mut max_val = 0.0_f32;
    // test 40hz sine wave at +10dB
    for i in 0..48000 {
        let t = i as f32 / 48000.0;
        let sample = (t * 40.0 * 2.0 * PI).sin() * 3.16; // +10dB
        let out = limiter.process(sample);
        if i > 4800 { // skip 100ms
            if out.abs() > max_val { max_val = out.abs(); }
        }
    }
    println!("Max peak: {}", max_val);
}
