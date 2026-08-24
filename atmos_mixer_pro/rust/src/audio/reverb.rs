pub struct DelayLine {
    buffer: Vec<f32>,
    write_idx: usize,
}

impl DelayLine {
    pub fn new(len: usize) -> Self {
        Self {
            buffer: vec![0.0; len],
            write_idx: 0,
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let out = self.buffer[self.write_idx];
        self.buffer[self.write_idx] = input;
        self.write_idx = (self.write_idx + 1) % self.buffer.len();
        out
    }
}

pub struct VirtualRoomReverb {
    delays: Vec<DelayLine>,
    feedback_matrix: [[f32; 4]; 4],
    decay: f32,
    pub mix: f32,
}

impl VirtualRoomReverb {
    pub fn new(sample_rate: f32) -> Self {
        let lengths = [
            (0.0297 * sample_rate) as usize,
            (0.0371 * sample_rate) as usize,
            (0.0411 * sample_rate) as usize,
            (0.0437 * sample_rate) as usize,
        ];
        
        let delays = lengths.iter().map(|&l| DelayLine::new(l)).collect();
        
        let q = 0.5;
        let feedback_matrix = [
            [q, q, q, q],
            [q, -q, q, -q],
            [q, q, -q, -q],
            [q, -q, -q, q],
        ];

        Self {
            delays,
            feedback_matrix,
            decay: 0.5,
            mix: 0.0,
        }
    }

    pub fn set_params(&mut self, decay: f32, mix: f32) {
        self.decay = decay.clamp(0.0, 0.99);
        self.mix = mix.clamp(0.0, 1.0);
    }

    pub fn process_stereo(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        if self.mix <= 0.0 {
            return (in_l, in_r);
        }

        let input = (in_l + in_r) * 0.5;

        let mut d_out = [0.0; 4];
        let mut out_sum_l = 0.0;
        let mut out_sum_r = 0.0;
        let mut next_inputs = [0.0; 4];
        
        for i in 0..4 {
            let out = self.delays[i].process(0.0);
            d_out[i] = out;
            if i % 2 == 0 {
                out_sum_l += out;
            } else {
                out_sum_r += out;
            }
        }

        for i in 0..4 {
            let mut feedback = 0.0;
            for j in 0..4 {
                feedback += self.feedback_matrix[i][j] * d_out[j];
            }
            next_inputs[i] = input + feedback * self.decay;
            let idx = (self.delays[i].write_idx + self.delays[i].buffer.len() - 1) % self.delays[i].buffer.len();
            self.delays[i].buffer[idx] = next_inputs[i];
        }

        let out_l = (in_l * (1.0 - self.mix)) + (out_sum_l * 0.5 * self.mix);
        let out_r = (in_r * (1.0 - self.mix)) + (out_sum_r * 0.5 * self.mix);

        (out_l, out_r)
    }
}
