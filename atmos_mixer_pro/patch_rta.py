import re

with open("rust/src/audio/rta.rs", "r") as f:
    content = f.read()

# Replace struct fields
content = content.replace("planner: rustfft::FftPlanner<f32>,", "fft: std::sync::Arc<dyn rustfft::Fft<f32>>,")

# Update new()
new_code = """pub fn new() -> Self {
        let mut planner = rustfft::FftPlanner::new();
        let fft = planner.plan_fft_forward(RTA_FFT_SIZE);
        
        let mut hann_window = vec![0.0; RTA_FFT_SIZE];
        for i in 0..RTA_FFT_SIZE {
            hann_window[i] = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (RTA_FFT_SIZE - 1) as f32).cos());
        }

        Self {
            ring_buffer: vec![0.0; RTA_FFT_SIZE],
            write_idx: 0,
            samples_since_last_fft: 0,
            hann_window,
            fft,
            input_buffer: vec![Complex { re: 0.0, im: 0.0 }; RTA_FFT_SIZE],
            scratch_buffer: vec![Complex { re: 0.0, im: 0.0 }; RTA_FFT_SIZE],
            fft_magnitudes: Arc::new(RwLock::new(vec![0.0; RTA_BIN_COUNT])),
        }
    }"""
new_start = content.find("pub fn new() -> Self {")
new_end = content.find("}\n\n    pub fn get_magnitudes", new_start)
content = content[:new_start] + new_code + content[new_end:]

# Update process_samples() inside RtaAnalyzer
content = content.replace("let fft = self.planner.plan_fft_forward(RTA_FFT_SIZE);", "")
content = content.replace("fft.process_with_scratch(&mut self.input_buffer, &mut self.scratch_buffer);", "self.fft.process_with_scratch(&mut self.input_buffer, &mut self.scratch_buffer);")

with open("rust/src/audio/rta.rs", "w") as f:
    f.write(content)
