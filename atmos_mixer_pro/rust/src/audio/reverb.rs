#[derive(Clone)]
pub struct DelayLine {
    buffer: Vec<f32>,
    write_idx: usize,
    delay_samples: usize,
}

impl DelayLine {
    pub fn new(max_len: usize) -> Self {
        Self {
            buffer: vec![0.0; max_len],
            write_idx: 0,
            delay_samples: max_len.saturating_sub(1).max(1),
        }
    }

    pub fn set_delay(&mut self, samples: usize) {
        self.delay_samples = samples.clamp(1, self.buffer.len() - 1);
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let read_idx = (self.write_idx + self.buffer.len() - self.delay_samples) % self.buffer.len();
        let out = self.buffer[read_idx];
        self.buffer[self.write_idx] = input;
        self.write_idx = (self.write_idx + 1) % self.buffer.len();
        out
    }
}

#[derive(Clone)]
pub struct VirtualRoomReverb {
    delays: Vec<DelayLine>,
    feedback_matrix: [[f32; 4]; 4],
    pub decay: f32,
    pub mix: f32,
    pub room_size: f32,
    pub pre_delay_ms: f32,
    pub damp: f32,
    pub density: f32,
    pub is_enabled: bool,
    
    base_lengths: [f32; 4],
    lpf_states: [f32; 4],
    allpass_delays: Vec<DelayLine>,
    pre_delay_line: DelayLine,
    sample_rate: f32,
}

impl VirtualRoomReverb {
    pub fn new(sample_rate: f32) -> Self {
        let base_lengths = [
            0.0297 * sample_rate, 
            0.0371 * sample_rate, 
            0.0411 * sample_rate, 
            0.0437 * sample_rate
        ];
        
        let mut delays = Vec::new();
        for &l in &base_lengths {
            // Allocate for up to 3x room size
            delays.push(DelayLine::new((l * 3.0) as usize));
        }

        let mut allpass_delays = Vec::new();
        // 4 allpass filters for diffusion
        let ap_lengths = [
            0.0051 * sample_rate, 
            0.0126 * sample_rate, 
            0.0100 * sample_rate, 
            0.0077 * sample_rate, 
        ];
        for &l in &ap_lengths {
            allpass_delays.push(DelayLine::new((l * 2.0) as usize));
        }
        
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
            room_size: 1.0,
            pre_delay_ms: 0.0,
            damp: 0.5,
            density: 1.0,
            is_enabled: true,
            
            base_lengths,
            lpf_states: [0.0; 4],
            allpass_delays,
            pre_delay_line: DelayLine::new((sample_rate * 0.5) as usize),
            sample_rate,
        }
    }

    pub fn set_params(&mut self, decay: f32, mix: f32) {
        self.decay = decay.clamp(0.0, 0.99);
        self.mix = mix.clamp(0.0, 1.0);
    }

    fn update_dsp_params(&mut self) {
        let pd_samples = ((self.pre_delay_ms / 1000.0) * self.sample_rate).max(1.0) as usize;
        self.pre_delay_line.set_delay(pd_samples);
        let size_mult = self.room_size.clamp(0.1, 3.0);
        for i in 0..4 {
            let samples = (self.base_lengths[i] * size_mult) as usize;
            self.delays[i].set_delay(samples);
        }
    }

    fn process_allpass(&mut self, input: f32, idx: usize) -> f32 {
        // Density controls the allpass coefficient
        let g = self.density.clamp(0.0, 0.99) * 0.7; // Max g = 0.7
        if g <= 0.01 { return input; }
        
        // y[n] = -g * x[n] + x[n-D] + g * y[n-D]
        // This can be done by:
        // delay_in = x[n] + g * delay_out
        // y[n] = -g * x[n] + delay_out
        
        // Wait, DelayLine process reads first, then writes. But DelayLine process takes input and returns output.
        // We can't do delay_in easily without splitting read and write.
        // Let's just use the current delay line process:
        // Read delay_out:
        let delay_out = self.allpass_delays[idx].process(0.0);
        
        let v = input - g * delay_out;
        let out = g * v + delay_out;
        
        // Write back v to the delay line (overwrite what was just written by process(0.0))
        let write_idx = (self.allpass_delays[idx].write_idx + self.allpass_delays[idx].buffer.len() - 1) % self.allpass_delays[idx].buffer.len();
        self.allpass_delays[idx].buffer[write_idx] = v;
        
        out
    }

    pub fn process_stereo(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        if self.mix <= 0.0 || !self.is_enabled {
            return (in_l, in_r);
        }

        self.update_dsp_params();

        let input_mono = (in_l + in_r) * 0.5;
        let mut input = self.pre_delay_line.process(input_mono);
        
        // Apply diffusion (density)
        for i in 0..4 {
            input = self.process_allpass(input, i);
        }

        let mut d_out = [0.0; 4];
        let mut out_sum_l = 0.0;
        let mut out_sum_r = 0.0;
        
        for i in 0..4 {
            let mut out = self.delays[i].process(0.0);
            
            // Damp (Lowpass filter in the feedback loop)
            // LPF formula: y[n] = y[n-1] + alpha * (x[n] - y[n-1])
            // Damp ranges from 0 (no damp) to 1 (max damp)
            let damp_coeff = self.damp.clamp(0.0, 1.0) * 0.5; // Max damp = 0.5 to keep it stable
            self.lpf_states[i] += damp_coeff * (out - self.lpf_states[i]);
            out = self.lpf_states[i] * damp_coeff + out * (1.0 - damp_coeff);
            
            d_out[i] = out;
            
            if i % 2 == 0 {
                out_sum_l += out;
            } else {
                out_sum_r += out;
            }
        }

        let mut next_inputs = [0.0; 4];
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

    pub fn process_mono(&mut self, input: f32) -> f32 {
        if self.mix <= 0.0 || !self.is_enabled {
            return input;
        }

        self.update_dsp_params();
        
        let delayed_input = self.pre_delay_line.process(input);
        let mut diffused_input = delayed_input;
        for i in 0..4 {
            diffused_input = self.process_allpass(diffused_input, i);
        }

        let mut d_out = [0.0; 4];
        let mut out_sum = 0.0;
        
        for i in 0..4 {
            let mut out = self.delays[i].process(0.0);
            
            let damp_coeff = self.damp.clamp(0.0, 1.0) * 0.5;
            self.lpf_states[i] += damp_coeff * (out - self.lpf_states[i]);
            out = self.lpf_states[i] * damp_coeff + out * (1.0 - damp_coeff);
            
            d_out[i] = out;
            out_sum += out;
        }

        let mut next_inputs = [0.0; 4];
        for i in 0..4 {
            let mut feedback = 0.0;
            for j in 0..4 {
                feedback += self.feedback_matrix[i][j] * d_out[j];
            }
            next_inputs[i] = diffused_input + feedback * self.decay;
            let idx = (self.delays[i].write_idx + self.delays[i].buffer.len() - 1) % self.delays[i].buffer.len();
            self.delays[i].buffer[idx] = next_inputs[i];
        }

        (input * (1.0 - self.mix)) + (out_sum * 0.25 * self.mix)
    }
}
