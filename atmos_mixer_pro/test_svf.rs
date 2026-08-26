use std::f32::consts::PI;

pub enum EqType { Bell, LowShelf, HighShelf, LowCut, HighCut, Notch }

#[derive(Clone)]
pub struct SvfFilter {
    ic1eq: f32,
    ic2eq: f32,
    a1: f32, a2: f32, a3: f32,
    m0: f32, m1: f32, m2: f32,
}

impl SvfFilter {
    pub fn new() -> Self {
        Self { ic1eq: 0.0, ic2eq: 0.0, a1: 0.0, a2: 0.0, a3: 0.0, m0: 1.0, m1: 0.0, m2: 0.0 }
    }
    pub fn update_coefficients(&mut self, eq_type: &EqType, fs: f32, freq: f32, q: f32, gain_db: f32) {
        let safe_q = q.max(0.01);
        let safe_freq = freq.clamp(10.0, fs * 0.49);
        let g = (PI * safe_freq / fs).tan();
        let k = 1.0 / safe_q;

        match eq_type {
            EqType::LowShelf => {
                let a = 10.0f32.powf(gain_db / 40.0);
                self.a1 = 1.0 / (1.0 + g * (k / a) + g * g);
                self.a2 = g * self.a1;
                self.a3 = g * self.a2;
                self.m0 = 1.0;
                self.m1 = k * (a - 1.0 / a);
                self.m2 = a * a - 1.0;
            }
            _ => {}
        }
    }
    pub fn process(&mut self, v0: f32) -> f32 {
        let v3 = v0 - self.ic2eq;
        let v1 = self.a1 * self.ic1eq + self.a2 * v3;
        let v2 = self.ic2eq + self.a2 * self.ic1eq + self.a3 * v3;
        self.ic1eq = 2.0 * v1 - self.ic1eq;
        self.ic2eq = 2.0 * v2 - self.ic2eq;
        self.m0 * v0 + self.m1 * v1 + self.m2 * v2
    }
}

fn main() {
    let mut filter = SvfFilter::new();
    filter.update_coefficients(&EqType::LowShelf, 48000.0, 100.0, 0.707, 10.0);
    let mut out = 0.0;
    for _ in 0..48000 {
        out = filter.process(1.0);
    }
    println!("out: {}", out);
}
