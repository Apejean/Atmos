use realfft::RealFftPlanner;
use rustfft::num_complex::Complex;
use std::sync::Arc;

pub struct BinauralChannel {
    ir_left_freq: Vec<Complex<f32>>,
    ir_right_freq: Vec<Complex<f32>>,
    fft_size: usize,
    input_buffer: Vec<f32>,
    overlap_add_left: Vec<f32>,
    overlap_add_right: Vec<f32>,
    planner: RealFftPlanner<f32>,
}

impl BinauralChannel {
    pub fn new(ir_left: &[f32], ir_right: &[f32], block_size: usize) -> Self {
        let ir_len = ir_left.len().max(ir_right.len());
        let fft_size = (block_size + ir_len - 1).next_power_of_two();
        
        let mut planner = RealFftPlanner::<f32>::new();
        let r2c = planner.plan_fft_forward(fft_size);
        
        let mut ir_left_padded = vec![0.0; fft_size];
        ir_left_padded[..ir_left.len()].copy_from_slice(ir_left);
        let mut ir_left_freq = r2c.make_output_vec();
        r2c.process(&mut ir_left_padded, &mut ir_left_freq).unwrap();

        let mut ir_right_padded = vec![0.0; fft_size];
        ir_right_padded[..ir_right.len()].copy_from_slice(ir_right);
        let mut ir_right_freq = r2c.make_output_vec();
        r2c.process(&mut ir_right_padded, &mut ir_right_freq).unwrap();

        Self {
            ir_left_freq,
            ir_right_freq,
            fft_size,
            input_buffer: vec![0.0; fft_size],
            overlap_add_left: vec![0.0; fft_size],
            overlap_add_right: vec![0.0; fft_size],
            planner,
        }
    }

    pub fn process_block(&mut self, input: &[f32], out_left: &mut [f32], out_right: &mut [f32]) {
        let block_size = input.len();
        self.input_buffer.fill(0.0);
        self.input_buffer[..block_size].copy_from_slice(input);

        let r2c = self.planner.plan_fft_forward(self.fft_size);
        let c2r = self.planner.plan_fft_inverse(self.fft_size);

        let mut input_freq = r2c.make_output_vec();
        r2c.process(&mut self.input_buffer, &mut input_freq).unwrap();

        let mut out_left_freq = vec![Complex::new(0.0, 0.0); input_freq.len()];
        let mut out_right_freq = vec![Complex::new(0.0, 0.0); input_freq.len()];

        for i in 0..input_freq.len() {
            out_left_freq[i] = input_freq[i] * self.ir_left_freq[i];
            out_right_freq[i] = input_freq[i] * self.ir_right_freq[i];
        }

        let mut out_left_time = vec![0.0; self.fft_size];
        let mut out_right_time = vec![0.0; self.fft_size];

        c2r.process(&mut out_left_freq, &mut out_left_time).unwrap();
        c2r.process(&mut out_right_freq, &mut out_right_time).unwrap();

        let scale = 1.0 / self.fft_size as f32;
        
        for i in 0..self.fft_size {
            self.overlap_add_left[i] += out_left_time[i] * scale;
            self.overlap_add_right[i] += out_right_time[i] * scale;
        }

        for i in 0..block_size {
            out_left[i] += self.overlap_add_left[i];
            out_right[i] += self.overlap_add_right[i];
        }

        self.overlap_add_left.drain(0..block_size);
        self.overlap_add_left.resize(self.fft_size, 0.0);
        
        self.overlap_add_right.drain(0..block_size);
        self.overlap_add_right.resize(self.fft_size, 0.0);
    }
}

pub struct VirtualMixRoomBinaural {
    channels: Vec<BinauralChannel>,
    pub enabled: bool,
    temp_channel_buffers: Vec<Vec<f32>>,
}

impl VirtualMixRoomBinaural {
    pub fn new(num_channels: usize, block_size: usize) -> Self {
        // Dummy HRTF for now
        let dummy_ir_left = vec![1.0, 0.0];
        let dummy_ir_right = vec![0.0, 1.0];
        
        let mut channels = Vec::new();
        let mut temp_channel_buffers = Vec::new();
        for _ in 0..num_channels {
            channels.push(BinauralChannel::new(&dummy_ir_left, &dummy_ir_right, block_size));
            temp_channel_buffers.push(vec![0.0; block_size]);
        }

        Self {
            channels,
            enabled: false,
            temp_channel_buffers,
        }
    }

    pub fn process_interleaved(&mut self, output: &mut [f32], out_channels: usize) {
        if !self.enabled || out_channels < 2 { return; }
        
        let frames = output.len() / out_channels;
        let num_ch = self.channels.len().min(out_channels);
        
        // Ensure temp buffers are correct size
        for buf in &mut self.temp_channel_buffers {
            if buf.len() < frames {
                buf.resize(frames, 0.0);
            }
        }
        
        // De-interleave
        for ch in 0..num_ch {
            for frame in 0..frames {
                self.temp_channel_buffers[ch][frame] = output[frame * out_channels + ch];
            }
        }
        
        let mut mix_left = vec![0.0; frames];
        let mut mix_right = vec![0.0; frames];
        
        // Process each channel
        for ch in 0..num_ch {
            self.channels[ch].process_block(&self.temp_channel_buffers[ch][..frames], &mut mix_left, &mut mix_right);
        }
        
        // Interleave back into output (overwrite Ch0 and Ch1, zero others)
        for frame in 0..frames {
            output[frame * out_channels + 0] = mix_left[frame];
            output[frame * out_channels + 1] = mix_right[frame];
            for ch in 2..out_channels {
                output[frame * out_channels + ch] = 0.0;
            }
        }
    }
}
