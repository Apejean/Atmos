use crate::audio::svf::SvfFilter;
use crate::common::config::EqType;

/// 재생 중 주파수 변경을 몇 샘플마다 계수에 반영할지(제어율).
/// 블록 단위로 반영하면 버퍼가 클 때(2048프레임 = 43ms) 한 번에 크게 뛰어 딸깍 소리가 난다.
const COEFF_UPDATE_INTERVAL: u32 = 32;
/// 재생 중 주파수 변경 스무딩 시정수(초). 로그 주파수 영역 1차 저역통과.
const FREQ_SMOOTHING_TAU_S: f32 = 0.03;

// 프로덕션 베이스 매니지먼트(서브우퍼 크로스오버)에서 실제로 사용되는 LR24 구현.
// (`AudioMixer::crossovers`, mixer.rs process()). `audio::svf::LinkwitzRiley24`는
// 별개의 범용 테스트용 구현이므로 혼동하지 말 것.
#[derive(Clone)]
pub struct LinkwitzRiley24 {
    hpf1: SvfFilter,
    hpf2: SvfFilter,
    lpf1: SvfFilter,
    lpf2: SvfFilter,
    /// 현재 계수에 반영된 주파수.
    pub crossover_freq: f32,
    target_freq: f32,
    fs: f32,
    update_countdown: u32,
}

impl LinkwitzRiley24 {
    pub fn new() -> Self {
        Self {
            hpf1: SvfFilter::new(),
            hpf2: SvfFilter::new(),
            lpf1: SvfFilter::new(),
            lpf2: SvfFilter::new(),
            crossover_freq: 80.0,
            target_freq: 80.0,
            fs: 48000.0,
            update_countdown: 0,
        }
    }

    /// 계수를 즉시 바꾼다. 오디오가 흐르기 전(초기화)에만 쓴다.
    pub fn set_crossover_freq(&mut self, freq: f32, fs: f32) {
        self.target_freq = freq;
        self.fs = fs;
        self.apply_freq(freq);
    }

    /// 재생 중 주파수 변경. 계수를 즉시 바꾸면 딸깍 소리가 나므로 `split()`이
    /// COEFF_UPDATE_INTERVAL 샘플마다 목표 쪽으로 조금씩 옮긴다(Law 3).
    pub fn set_target_freq(&mut self, freq: f32, fs: f32) {
        self.target_freq = freq;
        self.fs = fs;
    }

    /// 한 샘플을 (저역, 고역)으로 나눈다. 목표 주파수로 가는 중이면 계수도 갱신한다.
    #[inline(always)]
    pub fn split(&mut self, input: f32) -> (f32, f32) {
        if self.crossover_freq != self.target_freq {
            if self.update_countdown == 0 {
                self.update_countdown = COEFF_UPDATE_INTERVAL;
                self.step_toward_target();
            }
            self.update_countdown -= 1;
        }
        (self.process_low(input), self.process_high(input))
    }

    fn step_toward_target(&mut self) {
        let alpha =
            1.0 - (-(COEFF_UPDATE_INTERVAL as f32) / (FREQ_SMOOTHING_TAU_S * self.fs)).exp();
        let cur = self.crossover_freq.max(1.0).ln();
        let target = self.target_freq.max(1.0);
        let next = (cur + (target.ln() - cur) * alpha).exp();
        // 0.05Hz 안으로 들어오면 목표에 붙인다(끝없이 다가가며 계수를 다시 계산하지 않도록).
        let next = if (next - target).abs() < 0.05 { self.target_freq } else { next };
        self.apply_freq(next);
    }

    fn apply_freq(&mut self, freq: f32) {
        let fs = self.fs;
        self.crossover_freq = freq;
        // Butterworth Q is 0.707. Cascading two of them forms an LR4 filter.
        let q = std::f32::consts::FRAC_1_SQRT_2; 
        
        // svf.rs 규약: EqType::LowCut = HighPass(저역 컷), EqType::HighCut = LowPass(고역 컷)
        // hpf*는 하이패스여야 하므로 LowCut을, lpf*는 로우패스여야 하므로 HighCut을 사용한다.
        self.hpf1.update_coefficients(&EqType::LowCut, fs, freq, q, 0.0);
        self.hpf2.update_coefficients(&EqType::LowCut, fs, freq, q, 0.0);
        self.lpf1.update_coefficients(&EqType::HighCut, fs, freq, q, 0.0);
        self.lpf2.update_coefficients(&EqType::HighCut, fs, freq, q, 0.0);
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
