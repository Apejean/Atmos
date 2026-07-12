use biquad::*;

fn main() {
    let f0 = 1000.0.hz();
    let fs = 48000.0.hz();
    let coefs = Coefficients::<f32>::from_params(Type::LowPass, fs, f0, Q_BUTTERWORTH).unwrap();
    let mut filter = DirectForm1::<f32>::new(coefs);
    let out = filter.run(1.0);
}
