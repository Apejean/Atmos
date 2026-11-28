use std::sync::atomic::{AtomicU32, Ordering};

pub struct PeakLimiter {
    fast_attack_coef: f32,
    fast_release_coef: f32,
    slow_attack_coef: f32,
    slow_release_coef: f32,

    fast_envelope: f32,
    slow_envelope: f32,

    threshold: f32,
    knee_width: f32,

    delay_buffer: Vec<f32>,
    delay_index: usize,

    hold_counter: usize,
    hold_samples: usize,

    history: [f32; 3],

    /// 마지막으로 0이 아닌 입력 뒤 연속으로 들어온 0의 개수. 지연 줄 길이(그리고 history 3칸) 이상이면 지연 줄과
    /// history가 모두 0이다(`is_idle`). 새 리미터는 비어 있으므로 그 길이에서 시작한다.
    zero_run: usize,

    // 출력 음량 측정(3초 창, 간이 LUFS). 엔진 상태의 short_term_lufs로 보내기만 하고 소리는 바꾸지 않는다.
    pub short_term_lufs: f32,
    lufs_integration_buffer: Vec<f32>,
    lufs_index: usize,
    sum_sq: f32,
    lufs_decimation_counter: usize,

    // 현재 게인 리덕션(dB)을 f32 bits로 저장하는 락프리 텔레메트리 값.
    // 오디오 스레드(process)가 쓰고, UI/엔진 상태 스레드가 락 없이 읽는다(Law2 준수).
    gain_reduction_db_bits: AtomicU32,
}

impl PeakLimiter {
    pub fn new(sample_rate: f32, attack_ms: f32, release_ms: f32, threshold: f32) -> Self {
        // True Peak margin (-0.1 dB = ~0.9885, -0.3 dB = ~0.966)
        // Set actual threshold slightly lower to guarantee 0.99 limit is met
        let actual_threshold = threshold * 0.97;
        
        // 5ms lookahead buffer
        let delay_samples = ((sample_rate * 0.005) as usize).max(1);
        let delay_buffer = vec![0.0; delay_samples];
        
        // Oversampled detector rate (4x)
        let detector_rate = sample_rate * 4.0;
        
        // Attack should be very fast for limiter to catch peaks (e.g. 0.001ms)
        let fast_attack_sec = 0.0001; // 0.1ms for immediate clamping
        let fast_release_sec = 0.050; // 50ms for low frequency stability
        let slow_attack_sec = attack_ms / 1000.0;
        let slow_release_sec = release_ms / 1000.0; // typically 300~500ms
        
        let fast_attack_coef = if fast_attack_sec > 0.0 { (-1.0 / (fast_attack_sec * detector_rate)).exp() } else { 0.0 };
        let fast_release_coef = if fast_release_sec > 0.0 { (-1.0 / (fast_release_sec * detector_rate)).exp() } else { 0.0 };
        
        let slow_attack_coef = if slow_attack_sec > 0.0 { (-1.0 / (slow_attack_sec * detector_rate)).exp() } else { 0.0 };
        let slow_release_coef = if slow_release_sec > 0.0 { (-1.0 / (slow_release_sec * detector_rate)).exp() } else { 0.0 };

        Self {
            fast_attack_coef,
            fast_release_coef,
            slow_attack_coef,
            slow_release_coef,
            
            fast_envelope: 0.0,
            slow_envelope: 0.0,
            
            threshold: actual_threshold,
            knee_width: 0.005, // Make knee sharper for harder limiting
            
            delay_buffer,
            delay_index: 0,
            
            hold_counter: 0,
            hold_samples: delay_samples * 4,
            
            history: [0.0; 3],

            zero_run: delay_samples.max(3),

            short_term_lufs: -70.0,
            lufs_integration_buffer: vec![0.0; (sample_rate * 3.0) as usize], // 3 seconds integration window
            lufs_index: 0,
            sum_sq: 0.0,
            lufs_decimation_counter: 0,

            gain_reduction_db_bits: AtomicU32::new(0.0f32.to_bits()),
        }
    }

    /// 현재 게인 리덕션(dB, 항상 0 이상의 양수)을 락 없이 읽는다.
    /// 오디오 콜백이 아닌 스레드(엔진 상태 스트림 등)에서 호출해도 안전하다.
    pub fn current_gain_reduction_db(&self) -> f32 {
        f32::from_bits(self.gain_reduction_db_bits.load(Ordering::Relaxed))
    }

    /// 지금 0만 넣으면 출력이 0이고 게인이 1인가: 지연 줄과 history가 모두 0이고, 두 엔벨로프가 무릎 아래다.
    /// 참이면 무음 블록을 `skip_silent`로 넘겨도 `process(0.0)`을 부른 것과 결과가 같다.
    pub fn is_idle(&self) -> bool {
        let knee_lower = self.threshold - self.knee_width / 2.0;
        self.zero_run >= self.delay_buffer.len().max(3)
            && self.fast_envelope <= knee_lower
            && self.slow_envelope <= knee_lower
    }

    /// `is_idle()`일 때 `process(0.0)`을 `n`번 부른 것과 **비트까지 같게** 상태만 넘긴다(출력은 모두 0이다).
    /// 보간·지연 줄·무릎 게인·안전 클립을 건너뛰고, 입력 0일 때 실제로 바뀌는 것만 같은 연산으로 되풀이한다.
    /// - 엔벨로프(4배 검출이라 표본마다 4번): 입력 0이면 `coef × (env − 0) + 0` = `env × coef`다. 빠른 쪽은 hold가
    ///   남은 동안 그대로다. `powi` 한 번으로 줄이면 반올림이 달라져 쓰지 않는다(2026-11-28 리뷰).
    /// - LUFS 창: `process`와 같은 계산을 0으로 되풀이한다.
    pub fn skip_silent(&mut self, n: usize) {
        if n == 0 {
            return;
        }
        self.history = [0.0; 3];
        // 지연 줄은 모두 0이라 회전 위치와 상관없이 같지만, process와 같은 자리로 맞춰 둔다.
        self.delay_index = (self.delay_index + n) % self.delay_buffer.len();
        self.zero_run = self.zero_run.saturating_add(n);
        // 엔벨로프가 무릎 아래라 process도 게인 1 → 0 dB를 쓴다.
        self.gain_reduction_db_bits.store(0.0f32.to_bits(), Ordering::Relaxed);

        for _ in 0..n {
            for _ in 0..4 {
                if self.hold_counter > 0 {
                    self.hold_counter -= 1;
                } else {
                    self.fast_envelope *= self.fast_release_coef;
                }
                self.slow_envelope *= self.slow_release_coef;
            }

            let old_sq = self.lufs_integration_buffer[self.lufs_index];
            self.sum_sq -= old_sq;
            if self.sum_sq < 0.0 {
                self.sum_sq = 0.0;
            }
            self.lufs_integration_buffer[self.lufs_index] = 0.0;
            self.lufs_index += 1;
            if self.lufs_index >= self.lufs_integration_buffer.len() {
                self.lufs_index = 0;
            }
            if self.lufs_decimation_counter == 0 {
                let mean_sq = self.sum_sq / self.lufs_integration_buffer.len() as f32;
                self.short_term_lufs = if mean_sq > 1e-10 { -0.691 + 10.0 * mean_sq.log10() } else { -70.0 };
                self.lufs_decimation_counter = 127;
            } else {
                self.lufs_decimation_counter -= 1;
            }
        }
    }

    pub fn process(&mut self, sample: f32) -> f32 {
        if sample == 0.0 {
            self.zero_run = self.zero_run.saturating_add(1);
        } else {
            self.zero_run = 0;
        }
        // 4x Oversampling interpolation for True Peak detection (ISP)
        let mut max_abs = 0.0_f32;
        
        let x0 = self.history[0];
        let x1 = self.history[1];
        let x2 = self.history[2];
        let x3 = sample;
        
        // 4-tap Hermite interpolation
        let c0 = x1;
        let c1 = 0.5 * (x2 - x0);
        let c2 = x0 - 2.5 * x1 + 2.0 * x2 - 0.5 * x3;
        let c3 = 0.5 * (x3 - x0) + 1.5 * (x1 - x2);

        for i in 1..=4 {
            let t = i as f32 / 4.0;
            let interp = ((c3 * t + c2) * t + c1) * t + c0;
            let abs_val = interp.abs();
            if abs_val > max_abs {
                max_abs = abs_val;
            }
            
            // Run envelope follower at 4x rate
            if abs_val > self.fast_envelope {
                self.fast_envelope = self.fast_attack_coef * (self.fast_envelope - abs_val) + abs_val;
                self.hold_counter = self.hold_samples;
            } else {
                if self.hold_counter > 0 {
                    self.hold_counter -= 1;
                } else {
                    self.fast_envelope = self.fast_release_coef * (self.fast_envelope - abs_val) + abs_val;
                }
            }
            
            if abs_val > self.slow_envelope {
                self.slow_envelope = self.slow_attack_coef * (self.slow_envelope - abs_val) + abs_val;
            } else {
                self.slow_envelope = self.slow_release_coef * (self.slow_envelope - abs_val) + abs_val;
            }
        }
        
        self.history[0] = self.history[1];
        self.history[1] = self.history[2];
        self.history[2] = sample;
        
        // Remove .max(max_abs) to prevent instantaneous waveform tracing which causes LF distortion!
        // The hold_counter ensures that peaks are caught and held for exactly the duration of the delay line.
        let envelope = self.fast_envelope.max(self.slow_envelope);
        
        let mut gain = 1.0;
        let knee_lower = self.threshold - self.knee_width / 2.0;
        let knee_upper = self.threshold + self.knee_width / 2.0;
        
        if envelope > knee_lower {
            if envelope < knee_upper {
                let x = envelope - knee_lower;
                let target = knee_lower + (x * x) / (2.0 * self.knee_width);
                gain = target / envelope;
            } else {
                gain = self.threshold / envelope;
            }
        }

        // 게인 리덕션(dB) 텔레메트리 갱신: gain(0~1) -> dB로 환산해 락프리 저장.
        let gr_db = if gain < 1.0 { -20.0 * gain.log10() } else { 0.0 };
        self.gain_reduction_db_bits.store(gr_db.to_bits(), Ordering::Relaxed);

        let delayed_sample = self.delay_buffer[self.delay_index];
        self.delay_buffer[self.delay_index] = sample; // Need to store gain? No, store raw, apply gain to delayed
        
        self.delay_index += 1;
        if self.delay_index >= self.delay_buffer.len() {
            self.delay_index = 0;
        }
        
        // Use delay to allow envelope to catch up to the current sample
        let out = delayed_sample * gain;
        
        // Hard clip safeguard just in case
        let final_out = if out > 0.985 {
            0.985
        } else if out < -0.985 {
            -0.985
        } else {
            out
        };

        // 출력 음량 측정. 예전에는 이 값이 -14를 넘으면 채널마다 따로 소리를 낮췄다(R128 Autoguard).
        // 미터에 보이지 않고 채널별로 낮춰 공간 균형을 바꿔서 뺐다(2026-10-24 사용자 결정).
        // 스피커 보호는 위의 피크 리미터와 0.985 안전 클립이 한다.
        let sq = final_out * final_out;
        let old_sq = self.lufs_integration_buffer[self.lufs_index];
        self.sum_sq = self.sum_sq + sq - old_sq;
        // Avoid floating point drift
        if self.sum_sq < 0.0 { self.sum_sq = 0.0; }
        
        self.lufs_integration_buffer[self.lufs_index] = sq;
        self.lufs_index += 1;
        if self.lufs_index >= self.lufs_integration_buffer.len() {
            self.lufs_index = 0;
        }

        if self.lufs_decimation_counter == 0 {
            let mean_sq = self.sum_sq / self.lufs_integration_buffer.len() as f32;
            self.short_term_lufs = if mean_sq > 1e-10 {
                -0.691 + 10.0 * mean_sq.log10() // simplified LUFS approx
            } else {
                -70.0
            };
            self.lufs_decimation_counter = 127;
        } else {
            self.lufs_decimation_counter -= 1;
        }

        final_out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 같은 입력을 넣은 두 리미터를 비운 뒤, 한쪽은 `process(0.0)`을 n번, 다른 쪽은 `skip_silent(n)`으로 넘겨
    /// 내부 상태를 비트까지 비교한다. 출력 비교(`tests/test_limiter_silence_skip.rs`)는 엔벨로프가 무릎 아래라
    /// 작은 차이가 다음 소리에서 곧 잊혀 잡지 못하므로, 상태를 직접 본다.
    fn idle_pair(amp: f32) -> (PeakLimiter, PeakLimiter) {
        let mut a = PeakLimiter::new(48_000.0, 1.0, 500.0, 0.99);
        let mut b = PeakLimiter::new(48_000.0, 1.0, 500.0, 0.99);
        for i in 0..9_600 {
            let x = amp * (i as f32 * 0.13).sin();
            a.process(x);
            b.process(x);
        }
        while !a.is_idle() {
            a.process(0.0);
            b.process(0.0);
        }
        assert!(b.is_idle());
        (a, b)
    }

    fn assert_same_state(a: &PeakLimiter, b: &PeakLimiter, what: &str) {
        assert_eq!(a.fast_envelope.to_bits(), b.fast_envelope.to_bits(), "{what}: 빠른 엔벨로프 {} vs {}", a.fast_envelope, b.fast_envelope);
        assert_eq!(a.slow_envelope.to_bits(), b.slow_envelope.to_bits(), "{what}: 느린 엔벨로프 {} vs {}", a.slow_envelope, b.slow_envelope);
        assert_eq!(a.hold_counter, b.hold_counter, "{what}: hold");
        assert_eq!(a.history, b.history, "{what}: history");
        assert_eq!(a.delay_index, b.delay_index, "{what}: 지연 줄 위치");
        assert_eq!(a.zero_run, b.zero_run, "{what}: 연속 0 개수");
        assert_eq!(a.lufs_index, b.lufs_index, "{what}: LUFS 위치");
        assert_eq!(a.lufs_decimation_counter, b.lufs_decimation_counter, "{what}: LUFS 간격");
        assert_eq!(a.sum_sq.to_bits(), b.sum_sq.to_bits(), "{what}: LUFS 합");
        assert_eq!(a.short_term_lufs.to_bits(), b.short_term_lufs.to_bits(), "{what}: LUFS 값");
        assert_eq!(a.current_gain_reduction_db(), b.current_gain_reduction_db(), "{what}: 게인 리덕션");
    }

    /// a는 `process(0.0)`을 되풀이하고 b는 `skip_silent`으로 넘기며 블록마다 상태를 비교한다.
    fn check_skips(mut a: PeakLimiter, mut b: PeakLimiter, what: &str) {
        // 블록 길이 여러 가지, 마지막은 3초 LUFS 창을 몇 바퀴 넘긴다
        for n in [1usize, 480, 1024, 333, 200_000] {
            for _ in 0..n {
                assert_eq!(a.process(0.0), 0.0);
            }
            b.skip_silent(n);
            assert_same_state(&a, &b, &format!("{what}, {n}표본"));
            assert!(b.is_idle());
        }
    }

    #[test]
    fn 무음_건너뛰기는_0을_되풀이해_넣은_상태와_같다() {
        // 리미터가 일하던 큰 소리 뒤(엔벨로프가 무릎 바로 아래)와 작은 소리 뒤
        for amp in [1.4f32, 0.3] {
            let (a, b) = idle_pair(amp);
            check_skips(a, b, &format!("진폭 {amp}"));
        }
        // 조용할 때 들어온 작은 펄스 하나 뒤: 리미터가 비는 순간에도 hold가 남아 있다
        // (빠른 엔벨로프가 hold 동안 그대로인지 본다).
        let mut a = PeakLimiter::new(48_000.0, 1.0, 500.0, 0.99);
        let mut b = PeakLimiter::new(48_000.0, 1.0, 500.0, 0.99);
        for l in [&mut a, &mut b] {
            l.process(0.1);
            while !l.is_idle() {
                l.process(0.0);
            }
        }
        assert!(a.hold_counter > 0, "이 경우는 비는 순간 hold가 남아야 한다");
        check_skips(a, b, "작은 펄스");
    }

    #[test]
    fn 새_리미터는_비어_있고_0이_아닌_입력이_들어오면_비지_않는다() {
        let mut l = PeakLimiter::new(48_000.0, 1.0, 500.0, 0.99);
        assert!(l.is_idle());
        l.process(0.1);
        assert!(!l.is_idle(), "지연 줄에 소리가 남았는데 비었다고 했다");
        for _ in 0..239 {
            l.process(0.0);
        }
        assert!(!l.is_idle(), "지연 줄(240표본)이 아직 다 비지 않았다");
        l.process(0.0);
        assert!(l.is_idle());
    }
}
