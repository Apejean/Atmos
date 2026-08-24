import re

with open("rust/src/audio/binaural.rs", "r") as f:
    content = f.read()

# Replace struct fields
content = content.replace("planner: RealFftPlanner<f32>,", "r2c: std::sync::Arc<dyn realfft::RealToComplex<f32>>,\n    c2r: std::sync::Arc<dyn realfft::ComplexToReal<f32>>,")

# Update new()
new_start = content.find("pub fn new(ir_left: &[f32], ir_right: &[f32], block_size: usize) -> Self {")
new_end = content.find("input_freq,", new_start)
new_code = """pub fn new(ir_left: &[f32], ir_right: &[f32], block_size: usize) -> Self {
        let ir_len = ir_left.len().max(ir_right.len());
        let fft_size = (block_size + ir_len - 1).next_power_of_two();
        
        let mut planner = RealFftPlanner::<f32>::new();
        let r2c = planner.plan_fft_forward(fft_size);
        let c2r = planner.plan_fft_inverse(fft_size);
        
        let mut ir_left_padded = vec![0.0; fft_size];
        ir_left_padded[..ir_left.len()].copy_from_slice(ir_left);
        let mut ir_left_freq = r2c.make_output_vec();
        let _ = r2c.process(&mut ir_left_padded, &mut ir_left_freq);

        let mut ir_right_padded = vec![0.0; fft_size];
        ir_right_padded[..ir_right.len()].copy_from_slice(ir_right);
        let mut ir_right_freq = r2c.make_output_vec();
        let _ = r2c.process(&mut ir_right_padded, &mut ir_right_freq);

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
            r2c,
            c2r,
            input_freq,"""

content = content[:new_start] + new_code + content[new_end + len("input_freq,"):]

# Update update_hrtf()
content = content.replace("let r2c = self.planner.plan_fft_forward(self.fft_size);", "")
content = content.replace("r2c.process(&mut ir_left_padded, &mut self.target_ir_left_freq).unwrap();", "let _ = self.r2c.process(&mut ir_left_padded, &mut self.target_ir_left_freq);")
content = content.replace("r2c.process(&mut ir_right_padded, &mut self.target_ir_right_freq).unwrap();", "let _ = self.r2c.process(&mut ir_right_padded, &mut self.target_ir_right_freq);")

# Update process_block()
process_code = """    pub fn process_block(&mut self, input: &[f32], out_left: &mut [f32], out_right: &mut [f32]) {
        let block_size = input.len().min(self.fft_size);
        self.input_buffer.fill(0.0);
        self.input_buffer[..block_size].copy_from_slice(&input[..block_size]);

        let _ = self.r2c.process(&mut self.input_buffer, &mut self.input_freq);

        // Convolve with current IR
        for i in 0..self.input_freq.len() {
            self.out_left_freq[i] = self.input_freq[i] * self.ir_left_freq[i];
            self.out_right_freq[i] = self.input_freq[i] * self.ir_right_freq[i];
        }
        let _ = self.c2r.process(&mut self.out_left_freq, &mut self.out_left_time);
        let _ = self.c2r.process(&mut self.out_right_freq, &mut self.out_right_time);

        let scale = 1.0 / self.fft_size as f32;
        
        // If switching, convolve with target IR and crossfade
        if self.is_switching {
            for i in 0..self.input_freq.len() {
                self.out_left_freq[i] = self.input_freq[i] * self.target_ir_left_freq[i];
                self.out_right_freq[i] = self.input_freq[i] * self.target_ir_right_freq[i];
            }
            let _ = self.c2r.process(&mut self.out_left_freq, &mut self.out_left_time_target);
            let _ = self.c2r.process(&mut self.out_right_freq, &mut self.out_right_time_target);
            
            let fade_step = 1.0 / block_size as f32;
            
            for i in 0..block_size {
                self.crossfade_phase += fade_step;
                if self.crossfade_phase >= 1.0 {
                    self.crossfade_phase = 1.0;
                    self.is_switching = false;
                    self.ir_left_freq.copy_from_slice(&self.target_ir_left_freq);
                    self.ir_right_freq.copy_from_slice(&self.target_ir_right_freq);
                }
                
                let cur_l = self.out_left_time[i] * scale;
                let cur_r = self.out_right_time[i] * scale;
                let tar_l = self.out_left_time_target[i] * scale;
                let tar_r = self.out_right_time_target[i] * scale;
                
                let mix_l = cur_l * (1.0 - self.crossfade_phase) + tar_l * self.crossfade_phase;
                let mix_r = cur_r * (1.0 - self.crossfade_phase) + tar_r * self.crossfade_phase;
                
                out_left[i] = mix_l + self.overlap_add_left[i];
                out_right[i] = mix_r + self.overlap_add_right[i];
            }
        } else {
            for i in 0..block_size {
                out_left[i] = self.out_left_time[i] * scale + self.overlap_add_left[i];
                out_right[i] = self.out_right_time[i] * scale + self.overlap_add_right[i];
            }
        }
        
        // Update overlap buffers
        self.overlap_add_left.fill(0.0);
        self.overlap_add_right.fill(0.0);
        for i in block_size..self.fft_size {
            let src_i = i;
            let dst_i = i - block_size;
            if self.is_switching {
                let mix_l = (self.out_left_time[src_i] * (1.0 - self.crossfade_phase) + self.out_left_time_target[src_i] * self.crossfade_phase) * scale;
                let mix_r = (self.out_right_time[src_i] * (1.0 - self.crossfade_phase) + self.out_right_time_target[src_i] * self.crossfade_phase) * scale;
                self.overlap_add_left[dst_i] = mix_l;
                self.overlap_add_right[dst_i] = mix_r;
            } else {
                self.overlap_add_left[dst_i] = self.out_left_time[src_i] * scale;
                self.overlap_add_right[dst_i] = self.out_right_time[src_i] * scale;
            }
        }
    }"""
process_start = content.find("pub fn process_block(&mut self, input: &[f32], out_left: &mut [f32], out_right: &mut [f32]) {")
process_end = content.find("\n}", process_start) # find end of process_block
# Need to find the balancing closing brace
bracket_count = 0
for i in range(process_start, len(content)):
    if content[i] == '{':
        bracket_count += 1
    elif content[i] == '}':
        bracket_count -= 1
        if bracket_count == 0:
            process_end = i + 1
            break

content = content[:process_start] + process_code + content[process_end:]

# Update VirtualMixRoomBinaural::new block_size to 8192
content = content.replace("temp_channel_buffers.push(vec![0.0; block_size]);", "temp_channel_buffers.push(vec![0.0; 8192]);")
content = content.replace("mix_left: vec![0.0; block_size],", "mix_left: vec![0.0; 8192],")
content = content.replace("mix_right: vec![0.0; block_size],", "mix_right: vec![0.0; 8192],")

# Remove resize logic in process_interleaved
resize_logic = """        // Ensure temp buffers are correct size
        for buf in &mut self.temp_channel_buffers {
            if buf.len() < frames {
                buf.resize(frames, 0.0); 
            }
        }
        if self.mix_left.len() < frames {
            self.mix_left.resize(frames, 0.0);
            self.mix_right.resize(frames, 0.0);
        }"""
content = content.replace(resize_logic, "        let frames = frames.min(8192);")

# Replace vec with array
content = content.replace("let dummy_ir_left = vec![(0.5 - shift).max(0.0), 0.0];", "let dummy_ir_left = [(0.5 - shift).max(0.0), 0.0];")
content = content.replace("let dummy_ir_right = vec![(0.5 + shift).max(0.0), 1.0];", "let dummy_ir_right = [(0.5 + shift).max(0.0), 1.0];")
content = content.replace("let dummy_ir_left = vec![1.0, 0.0];", "let dummy_ir_left = [1.0, 0.0];")
content = content.replace("let dummy_ir_right = vec![0.0, 1.0];", "let dummy_ir_right = [0.0, 1.0];")

with open("rust/src/audio/binaural.rs", "w") as f:
    f.write(content)
