use realfft::RealFftPlanner;
use rustfft::num_complex::Complex;

pub struct BinauralChannel {
    ir_left_freq: Vec<Complex<f32>>,
    ir_right_freq: Vec<Complex<f32>>,
    target_ir_left_freq: Vec<Complex<f32>>,
    target_ir_right_freq: Vec<Complex<f32>>,
    fft_size: usize,
    input_buffer: Vec<f32>,
    overlap_add_left: Vec<f32>,
    overlap_add_right: Vec<f32>,
    planner: RealFftPlanner<f32>,
    input_freq: Vec<Complex<f32>>,
    out_left_freq: Vec<Complex<f32>>,
    out_right_freq: Vec<Complex<f32>>,
    out_left_time: Vec<f32>,
    out_right_time: Vec<f32>,
    out_left_time_target: Vec<f32>,
    out_right_time_target: Vec<f32>,
    crossfade_phase: f32,
    is_switching: bool,
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

        let target_ir_left_freq = ir_left_freq.clone();
        let target_ir_right_freq = ir_right_freq.clone();

        let input_freq = vec![Complex::new(0.0, 0.0); ir_left_freq.len()];
        let out_left_freq = vec![Complex::new(0.0, 0.0); ir_left_freq.len()];
        let out_right_freq = vec![Complex::new(0.0, 0.0); ir_left_freq.len()];
        let out_left_time = vec![0.0; fft_size];
        let out_right_time = vec![0.0; fft_size];
        let out_left_time_target = vec![0.0; fft_size];
        let out_right_time_target = vec![0.0; fft_size];

        Self {
            ir_left_freq,
            ir_right_freq,
            target_ir_left_freq,
            target_ir_right_freq,
            fft_size,
            input_buffer: vec![0.0; fft_size],
            overlap_add_left: vec![0.0; fft_size],
            overlap_add_right: vec![0.0; fft_size],
            planner,
            input_freq,
            out_left_freq,
            out_right_freq,
            out_left_time,
            out_right_time,
            out_left_time_target,
            out_right_time_target,
            crossfade_phase: 1.0,
            is_switching: false,
        }
    }
    
    pub fn update_hrtf(&mut self, new_ir_left: &[f32], new_ir_right: &[f32]) {
        let r2c = self.planner.plan_fft_forward(self.fft_size);
        
        let mut ir_left_padded = vec![0.0; self.fft_size];
        ir_left_padded[..new_ir_left.len().min(self.fft_size)].copy_from_slice(&new_ir_left[..new_ir_left.len().min(self.fft_size)]);
        r2c.process(&mut ir_left_padded, &mut self.target_ir_left_freq).unwrap();

        let mut ir_right_padded = vec![0.0; self.fft_size];
        ir_right_padded[..new_ir_right.len().min(self.fft_size)].copy_from_slice(&new_ir_right[..new_ir_right.len().min(self.fft_size)]);
        r2c.process(&mut ir_right_padded, &mut self.target_ir_right_freq).unwrap();
        
        self.is_switching = true;
        self.crossfade_phase = 0.0;
    }

    pub fn process_block(&mut self, input: &[f32], out_left: &mut [f32], out_right: &mut [f32]) {
        let block_size = input.len();
        self.input_buffer.fill(0.0);
        self.input_buffer[..block_size].copy_from_slice(input);

        let r2c = self.planner.plan_fft_forward(self.fft_size);
        let c2r = self.planner.plan_fft_inverse(self.fft_size);

        r2c.process(&mut self.input_buffer, &mut self.input_freq).unwrap();

        // Convolve with current IR
        for i in 0..self.input_freq.len() {
            self.out_left_freq[i] = self.input_freq[i] * self.ir_left_freq[i];
            self.out_right_freq[i] = self.input_freq[i] * self.ir_right_freq[i];
        }
        c2r.process(&mut self.out_left_freq, &mut self.out_left_time).unwrap();
        c2r.process(&mut self.out_right_freq, &mut self.out_right_time).unwrap();

        let scale = 1.0 / self.fft_size as f32;
        
        // If switching, convolve with target IR and crossfade
        if self.is_switching {
            for i in 0..self.input_freq.len() {
                self.out_left_freq[i] = self.input_freq[i] * self.target_ir_left_freq[i];
                self.out_right_freq[i] = self.input_freq[i] * self.target_ir_right_freq[i];
            }
            c2r.process(&mut self.out_left_freq, &mut self.out_left_time_target).unwrap();
            c2r.process(&mut self.out_right_freq, &mut self.out_right_time_target).unwrap();
            
            let fade_step = 1.0 / block_size as f32;
            
            for i in 0..block_size {
                self.crossfade_phase = (self.crossfade_phase + fade_step).min(1.0);
                // Equal power crossfade
                let p = self.crossfade_phase;
                let c_cos = (p * std::f32::consts::FRAC_PI_2).cos();
                let c_sin = (p * std::f32::consts::FRAC_PI_2).sin();
                
                self.out_left_time[i] = self.out_left_time[i] * c_cos + self.out_left_time_target[i] * c_sin;
                self.out_right_time[i] = self.out_right_time[i] * c_cos + self.out_right_time_target[i] * c_sin;
            }
            
            if self.crossfade_phase >= 1.0 {
                self.is_switching = false;
                self.ir_left_freq.copy_from_slice(&self.target_ir_left_freq);
                self.ir_right_freq.copy_from_slice(&self.target_ir_right_freq);
            }
        }
        
        for i in 0..self.fft_size {
            self.overlap_add_left[i] += self.out_left_time[i] * scale;
            self.overlap_add_right[i] += self.out_right_time[i] * scale;
        }

        for i in 0..block_size {
            out_left[i] += self.overlap_add_left[i];
            out_right[i] += self.overlap_add_right[i];
        }

        self.overlap_add_left.copy_within(block_size.., 0);
        self.overlap_add_left[self.fft_size - block_size..].fill(0.0);
        
        self.overlap_add_right.copy_within(block_size.., 0);
        self.overlap_add_right[self.fft_size - block_size..].fill(0.0);
    }
}

pub struct VirtualMixRoomBinaural {
    channels: Vec<BinauralChannel>,
    pub enabled: bool,
    temp_channel_buffers: Vec<Vec<f32>>,
    mix_left: Vec<f32>,
    mix_right: Vec<f32>,
    current_yaw: f32,
    current_pitch: f32,
    current_roll: f32,
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
            mix_left: vec![0.0; block_size],
            mix_right: vec![0.0; block_size],
            current_yaw: 0.0,
            current_pitch: 0.0,
            current_roll: 0.0,
        }
    }

    pub fn process_interleaved(&mut self, output: &mut [f32], out_channels: usize) {
        if !self.enabled || out_channels < 2 { return; }
        
        // Handle 3-DoF Head Tracking via GLOBAL_STATE
        let yaw = f32::from_bits(crate::core::state::GLOBAL_STATE.hrtf_yaw.load(std::sync::atomic::Ordering::Relaxed));
        let pitch = f32::from_bits(crate::core::state::GLOBAL_STATE.hrtf_pitch.load(std::sync::atomic::Ordering::Relaxed));
        let roll = f32::from_bits(crate::core::state::GLOBAL_STATE.hrtf_roll.load(std::sync::atomic::Ordering::Relaxed));
        
        if (yaw - self.current_yaw).abs() > 0.01 || (pitch - self.current_pitch).abs() > 0.01 || (roll - self.current_roll).abs() > 0.01 {
            self.current_yaw = yaw;
            self.current_pitch = pitch;
            self.current_roll = roll;
            
            // Generate synthetic HRTF shift based on yaw for demonstration
            // In a real SOFA implementation, we would look up the closest IRs based on (yaw, pitch, roll)
            let shift = yaw.sin() * 0.5; // -0.5 to 0.5
            let dummy_ir_left = vec![(0.5 - shift).max(0.0), 0.0];
            let dummy_ir_right = vec![(0.5 + shift).max(0.0), 1.0];
            
            for ch in &mut self.channels {
                ch.update_hrtf(&dummy_ir_left, &dummy_ir_right);
            }
        }
        
        let frames = output.len() / out_channels;
        let num_ch = self.channels.len().min(out_channels);
        
        // Ensure temp buffers are correct size
        for buf in &mut self.temp_channel_buffers {
            if buf.len() < frames {
                buf.resize(frames, 0.0); 
            }
        }
        if self.mix_left.len() < frames {
            self.mix_left.resize(frames, 0.0);
            self.mix_right.resize(frames, 0.0);
        }
        
        // De-interleave
        for ch in 0..num_ch {
            for frame in 0..frames {
                self.temp_channel_buffers[ch][frame] = output[frame * out_channels + ch];
            }
        }
        
        self.mix_left[..frames].fill(0.0);
        self.mix_right[..frames].fill(0.0);
        
        // Process each channel
        for ch in 0..num_ch {
            self.channels[ch].process_block(&self.temp_channel_buffers[ch][..frames], &mut self.mix_left[..frames], &mut self.mix_right[..frames]);
        }
        
        // Re-interleave
        for frame in 0..frames {
            output[frame * out_channels] = self.mix_left[frame];
            output[frame * out_channels + 1] = self.mix_right[frame];
        }
    }
}
