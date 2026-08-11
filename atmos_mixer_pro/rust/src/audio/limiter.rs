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
}

impl PeakLimiter {
    pub fn new(sample_rate: f32, attack_ms: f32, release_ms: f32, threshold: f32) -> Self {
        // True Peak margin (-0.1 dB = ~0.9885)
        let actual_threshold = threshold * 0.9885;
        
        // 5ms lookahead buffer
        let delay_samples = ((sample_rate * 0.005) as usize).max(1);
        let delay_buffer = vec![0.0; delay_samples];
        
        let fast_attack_sec = attack_ms / 1000.0;
        let fast_release_sec = 0.015; // 15ms
        let slow_attack_sec = attack_ms / 1000.0;
        let slow_release_sec = release_ms / 1000.0; // typically 300~500ms
        
        let fast_attack_coef = if fast_attack_sec > 0.0 { (-1.0 / (fast_attack_sec * sample_rate)).exp() } else { 0.0 };
        let fast_release_coef = if fast_release_sec > 0.0 { (-1.0 / (fast_release_sec * sample_rate)).exp() } else { 0.0 };
        
        let slow_attack_coef = if slow_attack_sec > 0.0 { (-1.0 / (slow_attack_sec * sample_rate)).exp() } else { 0.0 };
        let slow_release_coef = if slow_release_sec > 0.0 { (-1.0 / (slow_release_sec * sample_rate)).exp() } else { 0.0 };

        Self {
            fast_attack_coef,
            fast_release_coef,
            slow_attack_coef,
            slow_release_coef,
            
            fast_envelope: 0.0,
            slow_envelope: 0.0,
            
            threshold: actual_threshold,
            knee_width: 0.05, // 0.05 linear soft-knee width
            
            delay_buffer,
            delay_index: 0,
        }
    }

    pub fn process(&mut self, sample: f32) -> f32 {
        let abs_sample = sample.abs();
        
        // Two-stage envelope follower
        if abs_sample > self.fast_envelope {
            self.fast_envelope = self.fast_attack_coef * (self.fast_envelope - abs_sample) + abs_sample;
        } else {
            self.fast_envelope = self.fast_release_coef * (self.fast_envelope - abs_sample) + abs_sample;
        }
        
        if abs_sample > self.slow_envelope {
            self.slow_envelope = self.slow_attack_coef * (self.slow_envelope - abs_sample) + abs_sample;
        } else {
            self.slow_envelope = self.slow_release_coef * (self.slow_envelope - abs_sample) + abs_sample;
        }
        
        let envelope = self.fast_envelope.max(self.slow_envelope);
        
        let mut gain = 1.0;
        let knee_lower = self.threshold - self.knee_width / 2.0;
        let knee_upper = self.threshold + self.knee_width / 2.0;
        
        if envelope > knee_lower {
            if envelope < knee_upper {
                let x = envelope - knee_lower;
                let target = knee_lower + (x * x) / (2.0 * self.knee_width);
                gain = target / envelope;
            } else {
                gain = self.threshold / envelope;
            }
        }
        
        let delayed_sample = self.delay_buffer[self.delay_index];
        self.delay_buffer[self.delay_index] = sample;
        
        self.delay_index += 1;
        if self.delay_index >= self.delay_buffer.len() {
            self.delay_index = 0;
        }
        
        delayed_sample * gain
    }
}
