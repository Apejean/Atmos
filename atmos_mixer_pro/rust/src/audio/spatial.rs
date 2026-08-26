pub struct DbapMatrix {
    pub positions: Vec<(f32, f32, f32)>, // (x, y, z) coordinates for each channel
}

impl DbapMatrix {
    pub fn new(positions: Vec<(f32, f32, f32)>) -> Self {
        Self { positions }
    }

    /// Calculate gain for each speaker given a source position (x, y, z).
    /// Returns a vector of gains (0.0 to 1.0) matching the number of positions.
    pub fn calculate_gains(&self, source_x: f32, source_y: f32, source_z: f32, size: f32, output_gains: &mut [f32]) {
        let mut sum_w_sq = 0.0;

        let len = self.positions.len().min(output_gains.len());
        
        let r_blur = 0.1;

        // Inverse square law: W_i = 1 / (d_i^2 + (r_blur * (1.0 + size * 3.0))^2)
        for (i, gain) in output_gains.iter_mut().enumerate().take(len) {
            let (spk_x, spk_y, spk_z) = self.positions[i];
            let dx = source_x - spk_x;
            let dy = source_y - spk_y;
            let dz = source_z - spk_z;
            let dist_sq = dx * dx + dy * dy + dz * dz;
            let effective_blur = r_blur * (1.0 + size * 3.0);
            let w = 1.0 / (dist_sq + effective_blur * effective_blur);
            *gain = w;
            sum_w_sq += w * w;
        }

        // Power normalization
        let norm_factor = if sum_w_sq > 0.0 {
            1.0 / sum_w_sq.sqrt()
        } else {
            0.0
        };

        for gain in output_gains.iter_mut().take(len) {
            *gain *= norm_factor;
        }
    }
}

pub struct FractionalDelayLine {
    buffer: Vec<f32>,
    write_idx: usize,
}

impl FractionalDelayLine {
    pub fn new(max_delay_samples: usize) -> Self {
        Self {
            buffer: vec![0.0; max_delay_samples.max(1)],
            write_idx: 0,
        }
    }

    pub fn write_sample(&mut self, sample: f32) {
        self.buffer[self.write_idx] = sample;
        self.write_idx = (self.write_idx + 1) % self.buffer.len();
    }

    pub fn get_delayed(&self, delay_samples: f32) -> f32 {
        if delay_samples <= 0.0 {
            let read_idx = (self.write_idx + self.buffer.len() - 1) % self.buffer.len();
            return self.buffer[read_idx];
        }

        let delay_int = delay_samples.floor() as usize;
        let delay_frac = delay_samples - delay_int as f32;

        let len = self.buffer.len();
        if delay_int >= len {
            return 0.0;
        }

        let idx1 = (self.write_idx + len - delay_int - 1) % len;
        let idx2 = (idx1 + len - 1) % len;

        let val1 = self.buffer[idx1];
        let val2 = self.buffer[idx2];

        val1 + delay_frac * (val2 - val1)
    }
}

pub struct Spatializer3D {
    pub dbap: DbapMatrix,
    pub delay_lines: Vec<FractionalDelayLine>,
    pub sample_rate: f32,
    pub speed_of_sound: f32, // m/s
    temp_gains: Vec<f32>,
}

impl Spatializer3D {
    pub fn new(positions: Vec<(f32, f32, f32)>, sample_rate: f32) -> Self {
        let max_dist = 100.0; // max distance 100m
        let speed_of_sound = 343.0;
        let max_delay_samples = ((max_dist / speed_of_sound) * sample_rate) as usize + 10;
        
        let delay_lines = (0..positions.len())
            .map(|_| FractionalDelayLine::new(max_delay_samples))
            .collect();

        Self {
            temp_gains: vec![0.0; positions.len()],
            dbap: DbapMatrix::new(positions),
            delay_lines,
            sample_rate,
            speed_of_sound,
        }
    }

    /// Process a single input sample and accumulate into the output slice.
    pub fn process_sample(&mut self, input: f32, source_pos: (f32, f32, f32), size: f32, output: &mut [f32]) {
        self.dbap.calculate_gains(source_pos.0, source_pos.1, source_pos.2, size, &mut self.temp_gains);

        for (i, dl) in self.delay_lines.iter_mut().enumerate() {
            dl.write_sample(input);
            
            let (spk_x, spk_y, spk_z) = self.dbap.positions[i];
            let dx = source_pos.0 - spk_x;
            let dy = source_pos.1 - spk_y;
            let dz = source_pos.2 - spk_z;
            let dist_sq = dx * dx + dy * dy + dz * dz;
            let dist = dist_sq.sqrt();
            
            // Delay in seconds
            let delay_sec = dist / self.speed_of_sound;
            // Delay in samples
            let delay_samples = delay_sec * self.sample_rate;
            
            let delayed_sample = dl.get_delayed(delay_samples);
            
            if i < output.len() {
                output[i] += delayed_sample * self.temp_gains[i];
            }
        }
    }
}
