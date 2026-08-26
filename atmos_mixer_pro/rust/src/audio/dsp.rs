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
    
    impl Default for DcBlocker {
        fn default() -> Self {
            Self::new()
        }
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
        
        pub target_distance_meters: f32,
        pub current_distance_meters: f32,
        pub air_absorption: crate::audio::dsp::acoustic_physics::AirAbsorptionFilter,
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
                target_distance_meters: 0.0,
                current_distance_meters: 0.0,
                air_absorption: crate::audio::dsp::acoustic_physics::AirAbsorptionFilter::new(),
            }
        }
    
        pub fn update_delay_target(&mut self, target_delay_ms: f32) {
            self.target_delay_ms = target_delay_ms.clamp(0.0, 1000.0);
        }

        pub fn update_eq_targets(&mut self, target_bands: &[EqBand], fs: f32) {
            let limit = target_bands.len().min(MAX_EQ_BANDS);
            
            for (i, band) in target_bands.iter().enumerate().take(limit) {
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
    
        pub fn update_distance(&mut self, target_dist: f32) {
            self.target_distance_meters = target_dist;
        }

        #[inline(always)]
        pub fn process(&mut self, input: f32, fs: f32) -> f32 {
            // Distance and Delay parameter smoothing
            let diff_dist = self.target_distance_meters - self.current_distance_meters;
            if diff_dist.abs() > 0.01 {
                self.current_distance_meters += diff_dist * 0.005;
            } else {
                self.current_distance_meters = self.target_distance_meters;
            }

            self.air_absorption.set_distance(self.current_distance_meters, fs);

            let diff = self.target_delay_ms - self.current_delay_ms;
            
            // Physical limit for doppler shift: Max object speed ~34.3m/s (Mach 0.1)
            // 34.3m/s / 343m/s = 0.1 ratio
            // Max delay change per sample (in ms): 0.1 * 1000ms / fs
            let max_delta_ms_per_sample = 100.0 / fs;

            if diff.abs() > 0.001 {
                let mut delta = diff * 0.005; // Base smoothing factor
                // Clamp delta to physically plausible speeds to prevent extreme pitch bending
                delta = delta.clamp(-max_delta_ms_per_sample, max_delta_ms_per_sample);
                self.current_delay_ms += delta;
            } else {
                self.current_delay_ms = self.target_delay_ms;
            }
    
            // Write to delay buffer
            self.delay_buffer[self.delay_write_idx] = input;
            
            // Read from delay buffer with hermite interpolation
            let delay_samples = (self.current_delay_ms / 1000.0 * fs).clamp(0.0, (DELAY_BUFFER_SIZE - 4) as f32);
            let delay_int = delay_samples.floor() as usize;
            let delay_frac = delay_samples - delay_int as f32;
            
            // For hermite we need 4 points: x0, x1, x2, x3. We want to interpolate between x1 and x2.
            let read_idx_x0 = (self.delay_write_idx + DELAY_BUFFER_SIZE - delay_int + 1) % DELAY_BUFFER_SIZE;
            let read_idx_x1 = (self.delay_write_idx + DELAY_BUFFER_SIZE - delay_int) % DELAY_BUFFER_SIZE;
            let read_idx_x2 = (self.delay_write_idx + DELAY_BUFFER_SIZE - delay_int - 1) % DELAY_BUFFER_SIZE;
            let read_idx_x3 = (self.delay_write_idx + DELAY_BUFFER_SIZE - delay_int - 2) % DELAY_BUFFER_SIZE;
            
            let x0 = self.delay_buffer[read_idx_x0];
            let x1 = self.delay_buffer[read_idx_x1];
            let x2 = self.delay_buffer[read_idx_x2];
            let x3 = self.delay_buffer[read_idx_x3];
            
            let mut out = interpolate_hermite(x0, x1, x2, x3, delay_frac);
            prevent_denormal(&mut out);
            
            self.delay_write_idx = (self.delay_write_idx + 1) % DELAY_BUFFER_SIZE;
            
            // Apply EQs
            for filter in self.eq_filters.iter_mut() {
                out = filter.process(out, fs);
            }
            
            // Apply Air Absorption
            out = self.air_absorption.process(out);

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
        pub fn get_next(&mut self) -> f32 {
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

pub mod acoustic_physics {
    use crate::audio::svf::SvfFilter;
    use crate::common::config::EqType;

    pub struct FractionalDelayLine {
        buffer: Vec<f32>,
        write_idx: usize,
    }

    impl FractionalDelayLine {
        pub fn new(max_samples: usize) -> Self {
            Self {
                buffer: vec![0.0; max_samples],
                write_idx: 0,
            }
        }

        pub fn process(&mut self, input: f32, delay_samples: f32) -> f32 {
            let len = self.buffer.len() as f32;
            let mut read_idx = self.write_idx as f32 - delay_samples;
            if read_idx < 0.0 {
                read_idx += len;
            }

            let idx0 = read_idx as usize;
            let idx1 = (idx0 + 1) % self.buffer.len();
            let idx2 = (idx0 + 2) % self.buffer.len();
            
            // To do cubic hermite we need 4 points, but let's use 4 tap hermite
            let idx_m1 = (idx0 + self.buffer.len() - 1) % self.buffer.len();
            
            let x0 = self.buffer[idx_m1];
            let x1 = self.buffer[idx0];
            let x2 = self.buffer[idx1];
            let x3 = self.buffer[idx2];
            
            let frac = read_idx - idx0 as f32;
            
            let c0 = x1;
            let c1 = 0.5 * (x2 - x0);
            let c2 = x0 - 2.5 * x1 + 2.0 * x2 - 0.5 * x3;
            let c3 = 0.5 * (x3 - x0) + 1.5 * (x1 - x2);
            
            let out = ((c3 * frac + c2) * frac + c1) * frac + c0;
            
            self.buffer[self.write_idx] = input;
            self.write_idx = (self.write_idx + 1) % self.buffer.len();
            
            out
        }
    }

    #[derive(Clone)]
    pub struct AirAbsorptionFilter {
        lpf: SvfFilter,
        target_cutoff: f32,
        current_cutoff: f32,
    }

    impl AirAbsorptionFilter {
        pub fn new() -> Self {
            Self {
                lpf: SvfFilter::new(),
                target_cutoff: 20000.0,
                current_cutoff: 20000.0,
            }
        }

        pub fn set_distance(&mut self, dist_meters: f32, sample_rate: f32) {
            // Rough approximation: -3dB at 10kHz at 10m, -3dB at 5kHz at 20m etc.
            // Cutoff moves from 20000Hz down to around 2000Hz as distance increases
            let max_dist = 50.0; // Assume max significant effect at 50m
            let normalized_dist = (dist_meters / max_dist).clamp(0.0, 1.0);
            let min_cutoff = 2000.0;
            let max_cutoff = 20000.0;
            
            self.target_cutoff = max_cutoff - normalized_dist * (max_cutoff - min_cutoff);
            
            // Just update filter coefficients immediately if they differ significantly
            if (self.current_cutoff - self.target_cutoff).abs() > 10.0 {
                self.current_cutoff += 0.005 * (self.target_cutoff - self.current_cutoff);
                self.lpf.update_coefficients(&EqType::HighShelf, sample_rate, self.current_cutoff, 0.707, -normalized_dist * 24.0);
            }
        }

        pub fn process(&mut self, input: f32) -> f32 {
            self.lpf.process(input)
        }
    }
}
