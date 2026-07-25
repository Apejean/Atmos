pub mod dsp_utils {
    use crate::common::config::{EqBand, EqType};
    use crate::audio::svf::SvfFilter;
    
    pub const MAX_DSP_CHANNELS: usize = 24;
    pub const MAX_EQ_BANDS: usize = 8;
    pub const DELAY_BUFFER_SIZE: usize = 48000; // 1 second at 48kHz
    
    #[derive(Clone)]
    pub struct EqFilterState {
        pub enabled: bool,
        pub target_freq: f32,
        pub target_gain: f32,
        pub target_q: f32,
        pub current_freq: f32,
        pub current_gain: f32,
        pub current_q: f32,
        pub filter_type: EqType,
        pub filter: SvfFilter,
        pub update_counter: usize,
    }
    
    impl Default for EqFilterState {
        fn default() -> Self {
            Self::new()
        }
    }

    impl EqFilterState {
        pub fn new() -> Self {
            Self {
                enabled: false,
                target_freq: 1000.0,
                target_gain: 0.0,
                target_q: 0.707,
                current_freq: 1000.0,
                current_gain: 0.0,
                current_q: 0.707,
                filter_type: EqType::Bell,
                filter: SvfFilter::new(),
                update_counter: 0,
            }
        }
    
        pub fn update(&mut self, band: &EqBand, fs: f32) {
            let was_enabled = self.enabled;
            self.enabled = band.enabled;
            self.filter_type = band.filter_type.clone();
            
            self.target_freq = band.freq.clamp(20.0, fs / 2.0 * 0.95);
            self.target_q = band.q_factor.clamp(0.1, 10.0);
            self.target_gain = band.gain;
            
            if !was_enabled && self.enabled {
                self.current_freq = self.target_freq;
                self.current_gain = self.target_gain;
                self.current_q = self.target_q;
                self.recalculate(fs);
            }
        }

        fn recalculate(&mut self, fs: f32) {
            self.filter.update_coefficients(
                &self.filter_type,
                fs,
                self.current_freq,
                self.current_q,
                self.current_gain
            );
        }
    
        #[inline(always)]
        pub fn process(&mut self, input: f32, fs: f32) -> f32 {
            if !self.enabled {
                return input;
            }
            
            // Parameter smoothing
            let mut changed = false;
            let df = self.target_freq - self.current_freq;
            if df.abs() > 0.1 { self.current_freq += df * 0.05; changed = true; } else { self.current_freq = self.target_freq; }
            
            let dg = self.target_gain - self.current_gain;
            if dg.abs() > 0.01 { self.current_gain += dg * 0.05; changed = true; } else { self.current_gain = self.target_gain; }
            
            let dq = self.target_q - self.current_q;
            if dq.abs() > 0.01 { self.current_q += dq * 0.05; changed = true; } else { self.current_q = self.target_q; }
            
            if changed {
                self.update_counter += 1;
                // Recalculate every 16 samples to save CPU
                if self.update_counter >= 16 {
                    self.update_counter = 0;
                    self.recalculate(fs);
                }
            }

            self.filter.process(input)
        }
    }
    
    #[derive(Clone)]
    pub struct ChannelDspState {
        pub delay_buffer: Vec<f32>,
        pub delay_write_idx: usize,
        pub target_delay_ms: f32,
        pub current_delay_ms: f32,
        
        pub target_bands: Vec<EqBand>,
        pub current_bands: Vec<EqBand>,
        pub eq_filters: Vec<EqFilterState>,
    }
    
    impl Default for ChannelDspState {
        fn default() -> Self {
            Self::new()
        }
    }

    impl ChannelDspState {
        pub fn new() -> Self {
            let mut eq_filters = Vec::with_capacity(MAX_EQ_BANDS);
            for _ in 0..MAX_EQ_BANDS {
                eq_filters.push(EqFilterState::new());
            }
            Self {
                delay_buffer: vec![0.0; DELAY_BUFFER_SIZE],
                delay_write_idx: 0,
                target_delay_ms: 0.0,
                current_delay_ms: 0.0,
                target_bands: vec![EqBand::default(); MAX_EQ_BANDS],
                current_bands: vec![EqBand::default(); MAX_EQ_BANDS],
                eq_filters,
            }
        }
    
        pub fn update_delay_target(&mut self, target_delay_ms: f32) {
            self.target_delay_ms = target_delay_ms.clamp(0.0, 1000.0);
        }

        pub fn update_eq_targets(&mut self, target_bands: Vec<EqBand>, fs: f32) {
            let mut bands = target_bands;
            bands.truncate(MAX_EQ_BANDS);
            while bands.len() < MAX_EQ_BANDS {
                bands.push(EqBand::default());
            }
            
            for (i, band) in bands.iter().enumerate() {
                self.target_bands[i] = band.clone();
                self.current_bands[i] = band.clone();
                self.eq_filters[i].update(band, fs);
            }
        }
    
        #[inline(always)]
        pub fn process(&mut self, input: f32, fs: f32) -> f32 {
            // Delay parameter smoothing
            let diff = self.target_delay_ms - self.current_delay_ms;
            if diff.abs() > 0.001 {
                self.current_delay_ms += diff * 0.005; // Smoothing factor
            } else {
                self.current_delay_ms = self.target_delay_ms;
            }
    
            // Write to delay buffer
            self.delay_buffer[self.delay_write_idx] = input;
            
            // Read from delay buffer with fractional interpolation
            let delay_samples = (self.current_delay_ms / 1000.0 * fs).clamp(0.0, (DELAY_BUFFER_SIZE - 2) as f32);
            let delay_int = delay_samples.floor() as usize;
            let delay_frac = delay_samples - delay_int as f32;
            
            let read_idx1 = (self.delay_write_idx + DELAY_BUFFER_SIZE - delay_int) % DELAY_BUFFER_SIZE;
            let read_idx2 = (self.delay_write_idx + DELAY_BUFFER_SIZE - delay_int - 1) % DELAY_BUFFER_SIZE;
            
            let s1 = self.delay_buffer[read_idx1];
            let s2 = self.delay_buffer[read_idx2];
            
            let mut out = s1 + delay_frac * (s2 - s1);
            prevent_denormal(&mut out);
            
            self.delay_write_idx = (self.delay_write_idx + 1) % DELAY_BUFFER_SIZE;
            
            // Apply EQs
            for filter in self.eq_filters.iter_mut() {
                out = filter.process(out, fs);
            }
            
            out
        }
    }
    
    #[inline(always)]
    pub fn prevent_denormal(val: &mut f32) {
        let abs = val.abs();
        if abs > 0.0 && abs < 1e-15 {
            *val = 0.0;
        }
    }
}
