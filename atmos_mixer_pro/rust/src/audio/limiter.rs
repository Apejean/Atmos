pub struct PeakLimiter {
    attack_coef: f32,
    release_coef: f32,
    envelope: f32,
    threshold: f32,
}

impl PeakLimiter {
    pub fn new(sample_rate: f32, attack_ms: f32, release_ms: f32, threshold: f32) -> Self {
        let attack_sec = attack_ms / 1000.0;
        let release_sec = release_ms / 1000.0;
        
        let attack_coef = if attack_sec > 0.0 {
            (-1.0 / (attack_sec * sample_rate)).exp()
        } else {
            0.0
        };
        
        let release_coef = if release_sec > 0.0 {
            (-1.0 / (release_sec * sample_rate)).exp()
        } else {
            0.0
        };

        Self {
            attack_coef,
            release_coef,
            envelope: 0.0,
            threshold,
        }
    }

    pub fn process(&mut self, sample: f32) -> f32 {
        let abs_sample = sample.abs();
        
        // Fast attack, smooth release envelope
        if abs_sample > self.envelope {
            self.envelope = self.attack_coef * (self.envelope - abs_sample) + abs_sample;
        } else {
            self.envelope = self.release_coef * (self.envelope - abs_sample) + abs_sample;
        }

        let mut gain = 1.0;
        if self.envelope > self.threshold {
            gain = self.threshold / self.envelope;
        }

        sample * gain
    }
}
