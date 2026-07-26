pub mod dsp_utils {
    use crate::common::config::{EqBand, EqType};
    use crate::audio::svf::SvfFilter;
    
    pub const MAX_DSP_CHANNELS: usize = 128;
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
                self.recalculate(fs);
            }

            self.filter.process(input)
        }
    }
    
    #[derive(Clone)]
    pub struct DcBlocker {
        x1: f32,
        y1: f32,
        r: f32,
    }
    
    impl DcBlocker {
        pub fn new() -> Self {
            Self {
                x1: 0.0,
                y1: 0.0,
                r: 0.999934, // ~0.5Hz cutoff at 48kHz
            }
        }
        
        #[inline(always)]
        pub fn process(&mut self, input: f32, fs: f32) -> f32 {
            self.r = 1.0 - (2.0 * std::f32::consts::PI * 0.5 / fs);
            let mut output = input - self.x1 + self.r * self.y1;
            
            if output.abs() < 1e-15 {
                output = 0.0;
                self.y1 = 0.0;
            } else {
                self.y1 = output;
            }
            
            self.x1 = input;
            output
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
        pub dc_blocker: DcBlocker,
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
                dc_blocker: DcBlocker::new(),
            }
        }
    
        pub fn update_delay_target(&mut self, target_delay_ms: f32) {
            self.target_delay_ms = target_delay_ms.clamp(0.0, 1000.0);
        }

        pub fn update_eq_targets(&mut self, target_bands: &[EqBand], fs: f32) {
            let limit = target_bands.len().min(MAX_EQ_BANDS);
            
            for i in 0..limit {
                let band = &target_bands[i];
                self.target_bands[i] = band.clone();
                self.current_bands[i] = band.clone();
                self.eq_filters[i].update(band, fs);
            }
            
            // Fill remaining filters with defaults if target_bands is smaller than MAX_EQ_BANDS
            for i in limit..MAX_EQ_BANDS {
                let default_band = EqBand::default();
                self.target_bands[i] = default_band.clone();
                self.current_bands[i] = default_band.clone();
                self.eq_filters[i].update(&default_band, fs);
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
            
            // Apply DC Blocker
            out = self.dc_blocker.process(out, fs);
            
            out
        }
    }
    
    #[inline(always)]
    pub fn interpolate_hermite(x0: f32, x1: f32, x2: f32, x3: f32, t: f32) -> f32 {
        let diff = x1 - x2;
        let c1 = x2 - x0;
        let c3 = x3 - x0 + 3.0 * diff;
        let c2 = -(2.0 * diff + c1 + c3);
        0.5 * ((c3 * t + c2) * t + c1) * t + x1
    }

    pub struct GainSmoother {
        pub current: f32,
        pub target: f32,
        pub alpha: f32,
    }
    
    impl GainSmoother {
        pub fn new(initial_gain: f32, alpha: f32) -> Self {
            Self { current: initial_gain, target: initial_gain, alpha }
        }
        
        pub fn set_target(&mut self, new_target: f32) { self.target = new_target; }
        
        #[inline(always)]
        pub fn next(&mut self) -> f32 {
            self.current += self.alpha * (self.target - self.current);
            self.current
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
