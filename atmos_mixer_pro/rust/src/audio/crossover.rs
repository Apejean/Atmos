use crate::audio::svf::SvfFilter;
use crate::common::config::EqType;

#[derive(Clone)]
pub struct LinkwitzRiley24 {
    hpf1: SvfFilter,
    hpf2: SvfFilter,
    lpf1: SvfFilter,
    lpf2: SvfFilter,
    pub crossover_freq: f32,
}

impl LinkwitzRiley24 {
    pub fn new() -> Self {
        Self {
            hpf1: SvfFilter::new(),
            hpf2: SvfFilter::new(),
            lpf1: SvfFilter::new(),
            lpf2: SvfFilter::new(),
            crossover_freq: 80.0,
        }
    }

    pub fn set_crossover_freq(&mut self, freq: f32, fs: f32) {
        self.crossover_freq = freq;
        // Butterworth Q is 0.707. Cascading two of them forms an LR4 filter.
        let q = std::f32::consts::FRAC_1_SQRT_2; 
        
        self.hpf1.update_coefficients(&EqType::HighCut, fs, freq, q, 0.0);
        self.hpf2.update_coefficients(&EqType::HighCut, fs, freq, q, 0.0);
        self.lpf1.update_coefficients(&EqType::LowCut, fs, freq, q, 0.0);
        self.lpf2.update_coefficients(&EqType::LowCut, fs, freq, q, 0.0);
    }

    #[inline(always)]
    pub fn process_high(&mut self, input: f32) -> f32 {
        let v1 = self.hpf1.process(input);
        self.hpf2.process(v1)
    }

    #[inline(always)]
    pub fn process_low(&mut self, input: f32) -> f32 {
        let v1 = self.lpf1.process(input);
        self.lpf2.process(v1)
    }
}

impl Default for LinkwitzRiley24 {
    fn default() -> Self {
        Self::new()
    }
}
