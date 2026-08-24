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
    
    prev_sample: f32,

    // R128 Autoguard
    pub short_term_lufs: f32,
    pub autoguard_ducking: f32,
    lufs_integration_buffer: Vec<f32>,
    lufs_index: usize,
    sum_sq: f32,
}

impl PeakLimiter {
    pub fn new(sample_rate: f32, attack_ms: f32, release_ms: f32, threshold: f32) -> Self {
        // True Peak margin (-0.1 dB = ~0.9885, -0.3 dB = ~0.966)
        // Set actual threshold slightly lower to guarantee 0.99 limit is met
        let actual_threshold = threshold * 0.97;
        
        // 5ms lookahead buffer
        let delay_samples = ((sample_rate * 0.005) as usize).max(1);
        let delay_buffer = vec![0.0; delay_samples];
        
        // Oversampled detector rate (4x)
        let detector_rate = sample_rate * 4.0;
        
        // Attack should be very fast for limiter to catch peaks (e.g. 0.001ms)
        let fast_attack_sec = 0.0001; // 0.1ms for immediate clamping
        let fast_release_sec = 0.015; // 15ms
        let slow_attack_sec = attack_ms / 1000.0;
        let slow_release_sec = release_ms / 1000.0; // typically 300~500ms
        
        let fast_attack_coef = if fast_attack_sec > 0.0 { (-1.0 / (fast_attack_sec * detector_rate)).exp() } else { 0.0 };
        let fast_release_coef = if fast_release_sec > 0.0 { (-1.0 / (fast_release_sec * detector_rate)).exp() } else { 0.0 };
        
        let slow_attack_coef = if slow_attack_sec > 0.0 { (-1.0 / (slow_attack_sec * detector_rate)).exp() } else { 0.0 };
        let slow_release_coef = if slow_release_sec > 0.0 { (-1.0 / (slow_release_sec * detector_rate)).exp() } else { 0.0 };

        Self {
            fast_attack_coef,
            fast_release_coef,
            slow_attack_coef,
            slow_release_coef,
            
            fast_envelope: 0.0,
            slow_envelope: 0.0,
            
            threshold: actual_threshold,
            knee_width: 0.005, // Make knee sharper for harder limiting
            
            delay_buffer,
            delay_index: 0,
            
            prev_sample: 0.0,

            short_term_lufs: -70.0,
            autoguard_ducking: 1.0,
            lufs_integration_buffer: vec![0.0; (sample_rate * 3.0) as usize], // 3 seconds integration window
            lufs_index: 0,
            sum_sq: 0.0,
        }
    }

    pub fn process(&mut self, sample: f32) -> f32 {
        // 4x Oversampling interpolation for True Peak detection (ISP)
        let mut max_abs = 0.0_f32;
        
        // Simple linear interpolation for 4x oversampling points
        for i in 1..=4 {
            let frac = i as f32 / 4.0;
            let interp = self.prev_sample + (sample - self.prev_sample) * frac;
            let abs_val = interp.abs();
            if abs_val > max_abs {
                max_abs = abs_val;
            }
            
            // Run envelope follower at 4x rate
            if abs_val > self.fast_envelope {
                self.fast_envelope = self.fast_attack_coef * (self.fast_envelope - abs_val) + abs_val;
            } else {
                self.fast_envelope = self.fast_release_coef * (self.fast_envelope - abs_val) + abs_val;
            }
            
            if abs_val > self.slow_envelope {
                self.slow_envelope = self.slow_attack_coef * (self.slow_envelope - abs_val) + abs_val;
            } else {
                self.slow_envelope = self.slow_release_coef * (self.slow_envelope - abs_val) + abs_val;
            }
        }
        
        self.prev_sample = sample;
        
        // Absolute safety clamp to current max if envelopes are too slow
        let envelope = self.fast_envelope.max(self.slow_envelope).max(max_abs);
        
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
        self.delay_buffer[self.delay_index] = sample; // Need to store gain? No, store raw, apply gain to delayed
        
        self.delay_index += 1;
        if self.delay_index >= self.delay_buffer.len() {
            self.delay_index = 0;
        }
        
        // Use delay to allow envelope to catch up to the current sample
        let out = delayed_sample * gain;
        
        // Hard clip safeguard just in case
        let final_out = if out > 0.985 {
            0.985
        } else if out < -0.985 {
            -0.985
        } else {
            out
        };

        // R128 Autoguard Calculation
        let sq = final_out * final_out;
        let old_sq = self.lufs_integration_buffer[self.lufs_index];
        self.sum_sq = self.sum_sq + sq - old_sq;
        // Avoid floating point drift
        if self.sum_sq < 0.0 { self.sum_sq = 0.0; }
        
        self.lufs_integration_buffer[self.lufs_index] = sq;
        self.lufs_index += 1;
        if self.lufs_index >= self.lufs_integration_buffer.len() {
            self.lufs_index = 0;
        }

        let mean_sq = self.sum_sq / self.lufs_integration_buffer.len() as f32;
        self.short_term_lufs = if mean_sq > 1e-10 {
            -0.691 + 10.0 * mean_sq.log10() // simplified LUFS approx
        } else {
            -70.0
        };

        // Autoguard threshold (e.g. -14 LUFS)
        let autoguard_threshold = -14.0;
        let mut target_ducking = 1.0;
        if self.short_term_lufs > autoguard_threshold {
            let over = self.short_term_lufs - autoguard_threshold;
            target_ducking = 10.0_f32.powf(-over / 20.0);
        }

        // Smooth ducking change
        self.autoguard_ducking += 0.001 * (target_ducking - self.autoguard_ducking);

        final_out * self.autoguard_ducking
    }
}
