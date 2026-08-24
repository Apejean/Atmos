use std::f32::consts::PI;
use crate::common::config::EqType;

#[derive(Clone)]
pub struct SvfFilter {
    ic1eq: f32,
    ic2eq: f32,
    
    // Coefficients
    a1: f32,
    a2: f32,
    a3: f32,
    m0: f32,
    m1: f32,
    m2: f32,
}

impl Default for SvfFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl SvfFilter {
    pub fn new() -> Self {
        Self {
            ic1eq: 0.0,
            ic2eq: 0.0,
            a1: 0.0,
            a2: 0.0,
            a3: 0.0,
            m0: 1.0,
            m1: 0.0,
            m2: 0.0,
        }
    }

    pub fn update_coefficients(&mut self, eq_type: &EqType, fs: f32, freq: f32, q: f32, gain_db: f32) {
        let safe_q = q.max(0.01);
        let safe_freq = freq.clamp(10.0, fs * 0.49);
        let g = (PI * safe_freq / fs).tan();
        let k = 1.0 / safe_q;

        match eq_type {
            EqType::Bell => {
                let a = 10.0f32.powf(gain_db / 40.0);
                self.a1 = 1.0 / (1.0 + g * (k / a) + g * g);
                self.a2 = g * self.a1;
                self.a3 = g * self.a2;
                self.m0 = 1.0;
                self.m1 = k * (a - 1.0 / a);
                self.m2 = 0.0;
            }
            EqType::LowShelf => {
                let a = 10.0f32.powf(gain_db / 40.0);
                self.a1 = 1.0 / (1.0 + g * (k / a) + g * g);
                self.a2 = g * self.a1;
                self.a3 = g * self.a2;
                self.m0 = 1.0;
                self.m1 = k * (a - 1.0 / a);
                self.m2 = a * a - 1.0;
            }
            EqType::HighShelf => {
                let a = 10.0f32.powf(gain_db / 40.0);
                self.a1 = 1.0 / (1.0 + g * (k * a) + g * g);
                self.a2 = g * self.a1;
                self.a3 = g * self.a2;
                self.m0 = a * a;
                self.m1 = k * (1.0 / a - a) * a * a;
                self.m2 = 1.0 - a * a;
            }
            EqType::LowCut => { // HighPass Filter (Cuts low frequencies)
                self.a1 = 1.0 / (1.0 + g * k + g * g);
                self.a2 = g * self.a1;
                self.a3 = g * self.a2;
                self.m0 = 1.0;
                self.m1 = -k;
                self.m2 = -1.0;
            }
            EqType::HighCut => { // LowPass Filter (Cuts high frequencies)
                self.a1 = 1.0 / (1.0 + g * k + g * g);
                self.a2 = g * self.a1;
                self.a3 = g * self.a2;
                self.m0 = 0.0;
                self.m1 = 0.0;
                self.m2 = 1.0;
            }
            EqType::Notch => {
                self.a1 = 1.0 / (1.0 + g * k + g * g);
                self.a2 = g * self.a1;
                self.a3 = g * self.a2;
                self.m0 = 1.0;
                self.m1 = -k;
                self.m2 = 0.0;
            }
        }
    }

    #[inline(always)]
    pub fn process(&mut self, v0: f32) -> f32 {
        let v3 = v0 - self.ic2eq;
        let v1 = self.a1 * self.ic1eq + self.a2 * v3;
        let v2 = self.ic2eq + self.a2 * self.ic1eq + self.a3 * v3;

        self.ic1eq = 2.0 * v1 - self.ic1eq;
        self.ic2eq = 2.0 * v2 - self.ic2eq;

        self.prevent_denormal();

        self.m0 * v0 + self.m1 * v1 + self.m2 * v2
    }

    #[inline(always)]
    fn prevent_denormal(&mut self) {
        let abs1 = self.ic1eq.abs();
        if abs1 > 0.0 && abs1 < 1e-15 {
            self.ic1eq = 0.0;
        }
        let abs2 = self.ic2eq.abs();
        if abs2 > 0.0 && abs2 < 1e-15 {
            self.ic2eq = 0.0;
        }
    }
}

#[derive(Clone, Default)]
pub struct LinkwitzRiley24 {
    stage1: SvfFilter,
    stage2: SvfFilter,
}

impl LinkwitzRiley24 {
    pub fn new() -> Self {
        Self {
            stage1: SvfFilter::new(),
            stage2: SvfFilter::new(),
        }
    }

    pub fn set_coefficients(&mut self, filter_type: EqType, sample_rate: f32, freq: f32) {
        // Linkwitz-Riley 24dB/oct is formed by two cascaded 12dB/oct Butterworth filters
        // Q factor for Butterworth is 0.707 (1/sqrt(2))
        self.stage1.set_coefficients(filter_type, sample_rate, freq, 0.707, 0.0);
        self.stage2.set_coefficients(filter_type, sample_rate, freq, 0.707, 0.0);
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let y = self.stage1.process(input);
        self.stage2.process(y)
    }
}
