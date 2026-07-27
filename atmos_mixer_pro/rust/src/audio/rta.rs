use rustfft::{FftPlanner, num_complex::Complex};
use std::sync::Arc;
use parking_lot::RwLock;

pub const RTA_FFT_SIZE: usize = 2048;
pub const RTA_BIN_COUNT: usize = RTA_FFT_SIZE / 2;

/// A thread-safe RTA Analyzer designed to be written to from the audio callback
/// and polled from the Dart FFI frontend.
pub struct RtaAnalyzer {
    ring_buffer: Vec<f32>,
    write_idx: usize,
    hann_window: Vec<f32>,
    fft_magnitudes: Arc<RwLock<Vec<f32>>>,
    planner: FftPlanner<f32>,
    scratch_buffer: Vec<Complex<f32>>,
    input_buffer: Vec<Complex<f32>>,
    frames_since_last_fft: usize,
}

impl Default for RtaAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl RtaAnalyzer {
    pub fn new() -> Self {
        // Pre-compute Hann window
        let mut hann_window = vec![0.0; RTA_FFT_SIZE];
        for i in 0..RTA_FFT_SIZE {
            let n = i as f32;
            let n_minus_1 = (RTA_FFT_SIZE - 1) as f32;
            hann_window[i] = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * n / n_minus_1).cos());
        }

        Self {
            ring_buffer: vec![0.0; RTA_FFT_SIZE],
            write_idx: 0,
            hann_window,
            fft_magnitudes: Arc::new(RwLock::new(vec![-100.0; RTA_BIN_COUNT])),
            planner: FftPlanner::new(),
            scratch_buffer: vec![Complex { re: 0.0, im: 0.0 }; RTA_FFT_SIZE],
            input_buffer: vec![Complex { re: 0.0, im: 0.0 }; RTA_FFT_SIZE],
            frames_since_last_fft: 0,
        }
    }

    /// Push mono mixed samples from the master output into the ring buffer
    pub fn process_samples(&mut self, samples: &[f32]) {
        for &sample in samples {
            self.ring_buffer[self.write_idx] = sample;
            self.write_idx = (self.write_idx + 1) % RTA_FFT_SIZE;
            self.frames_since_last_fft += 1;
        }

        // Calculate FFT every N frames to save CPU (e.g., hop size = 512 for ~75% overlap)
        let hop_size = 512;
        if self.frames_since_last_fft >= hop_size {
            self.frames_since_last_fft = 0;
            self.compute_fft();
        }
    }

    fn compute_fft(&mut self) {
        // Unroll ring buffer and apply Hann window
        for i in 0..RTA_FFT_SIZE {
            let read_idx = (self.write_idx + i) % RTA_FFT_SIZE;
            let val = self.ring_buffer[read_idx] * self.hann_window[i];
            self.input_buffer[i] = Complex { re: val, im: 0.0 };
        }

        let fft = self.planner.plan_fft_forward(RTA_FFT_SIZE);
        fft.process_with_scratch(&mut self.input_buffer, &mut self.scratch_buffer);

        // Calculate magnitude in dBFS
        let mut mags = vec![0.0; RTA_BIN_COUNT];
        let norm_factor = 2.0 / RTA_FFT_SIZE as f32; // Normalization for single-sided spectrum
        
        // Window energy correction factor for Hann window
        let window_correction = 2.0;

        for i in 0..RTA_BIN_COUNT {
            let complex = self.input_buffer[i];
            let mag = (complex.re * complex.re + complex.im * complex.im).sqrt();
            let mut mag_norm = mag * norm_factor * window_correction;
            
            // DC and Nyquist bin corrections
            if i == 0 || i == RTA_BIN_COUNT - 1 {
                mag_norm /= 2.0;
            }

            let db = if mag_norm > 1e-7 {
                20.0 * mag_norm.log10()
            } else {
                -140.0
            };
            mags[i] = db.clamp(-140.0, 0.0);
        }

        // Store into RwLock for FFI to poll
        let mut lock = self.fft_magnitudes.write();
        *lock = mags;
    }

    /// Returns a clone of the thread-safe reference to the magnitude vector.
    pub fn get_magnitudes_arc(&self) -> Arc<RwLock<Vec<f32>>> {
        self.fft_magnitudes.clone()
    }
}
