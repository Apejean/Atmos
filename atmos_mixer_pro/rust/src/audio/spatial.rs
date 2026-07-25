pub struct DbapMatrix {
    pub positions: Vec<(f32, f32)>, // (x, y) coordinates for each channel
}

impl DbapMatrix {
    pub fn new(positions: Vec<(f32, f32)>) -> Self {
        Self { positions }
    }

    /// Calculate gain for each speaker given a source position (x, y).
    /// Returns a vector of gains (0.0 to 1.0) matching the number of positions.
    pub fn calculate_gains(&self, source_x: f32, source_y: f32) -> Vec<f32> {
        let mut weights = Vec::with_capacity(self.positions.len());
        let mut sum_w_sq = 0.0;

        // Inverse square law: W_i = 1 / (d_i^2 + 0.0001)
        for (spk_x, spk_y) in &self.positions {
            let dx = source_x - spk_x;
            let dy = source_y - spk_y;
            let dist_sq = dx * dx + dy * dy;
            let w = 1.0 / (dist_sq + 0.0001);
            weights.push(w);
            sum_w_sq += w * w;
        }

        // Power normalization
        let norm_factor = if sum_w_sq > 0.0 {
            1.0 / sum_w_sq.sqrt()
        } else {
            0.0
        };

        for w in &mut weights {
            *w *= norm_factor;
        }

        weights
    }
}
