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
        // Linear trapezoidal integrated SVF from Andy Simper (Cytomic)
        let a = 10.0f32.powf(gain_db / 40.0);
        let g = (PI * freq / fs).tan();
        let k = 1.0 / q;

        match eq_type {
            EqType::Bell => {
                self.a1 = 1.0 / (1.0 + g * (k / a) + g * g);
                self.a2 = g * self.a1;
                self.a3 = g * self.a2;
                self.m0 = 1.0;
                self.m1 = 0.0;
                self.m2 = k * (a - 1.0 / a);
            }
            EqType::LowShelf => {
                self.a1 = 1.0 / (1.0 + g * (k / a) + g * g);
                self.a2 = g * self.a1;
                self.a3 = g * self.a2;
                self.m0 = 1.0;
                self.m1 = a * a - 1.0;
                self.m2 = k * (a - 1.0 / a);
            }
            EqType::HighShelf => {
                let a = 10.0f32.powf(gain_db / 40.0);
                let g = (PI * freq / fs).tan();
                let k = 1.0 / q;
                self.a1 = 1.0 / (1.0 + g * (k * a) + g * g);
                self.a2 = g * self.a1;
                self.a3 = g * self.a2;
                self.m0 = a * a;
                self.m1 = 1.0 - a * a;
                self.m2 = k * (1.0 / a - a) * a * a;
            }
            EqType::LowCut => { // HighPass
                self.a1 = 1.0 / (1.0 + g * k + g * g);
                self.a2 = g * self.a1;
                self.a3 = g * self.a2;
                self.m0 = 1.0;
                self.m1 = -1.0;
                self.m2 = -k;
            }
            EqType::HighCut => { // LowPass
                self.a1 = 1.0 / (1.0 + g * k + g * g);
                self.a2 = g * self.a1;
                self.a3 = g * self.a2;
                self.m0 = 0.0;
                self.m1 = 1.0;
                self.m2 = 0.0;
            }
            EqType::Notch => {
                self.a1 = 1.0 / (1.0 + g * k + g * g);
                self.a2 = g * self.a1;
                self.a3 = g * self.a2;
                self.m0 = 1.0;
                self.m1 = 0.0;
                self.m2 = -k;
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
