pub mod dsp_utils {
    use crate::common::config::{EqBand, EqType};
    use crate::audio::svf::SvfFilter;
    
    pub const MAX_DSP_CHANNELS: usize = 128;
    pub const MAX_EQ_BANDS: usize = 8;
    pub const DELAY_BUFFER_SIZE: usize = 48000; // 1 second at 48kHz
    // 초기반사음 전용 링버퍼 크기: 48kHz 기준 200ms. DELAY_BUFFER_SIZE(1초, 메인 시간정렬용) 관례를
    // 그대로 따르되 용도(1차 반사만)에 맞게 축소. acoustic::EARLY_REFLECTION_MAX_DELAY_MS와 정합.
    pub const EARLY_REFLECTION_BUFFER_SIZE: usize = 9600;
    
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
    
    /// 초기반사음 탭 1개의 스무딩 상태(target/current 쌍). Law 3: 즉시 스냅 금지.
    #[derive(Clone, Copy, Default)]
    pub struct EarlyReflectionTapState {
        pub target_delay_ms: f32,
        pub current_delay_ms: f32,
        pub target_gain: f32,
        pub current_gain: f32,
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
        pub phase_invert: bool,
        pub target_gain_linear: f32,
        pub target_reverb_send: f32,
        pub current_reverb_send: f32,
        pub current_gain_linear: f32,
        pub reverb: crate::audio::reverb::VirtualRoomReverb,

        // 초기반사음(Image-Source 1차 반사) - 채널 고정 6슬롯, 사전 할당된 링버퍼만 사용(Law 1).
        pub early_ref_buffer: Vec<f32>,
        pub early_ref_write_idx: usize,
        pub taps: [EarlyReflectionTapState; crate::audio::acoustic::MAX_EARLY_REFLECTION_TAPS],
        pub target_early_ref_mix: f32,
        pub current_early_ref_mix: f32,
    }
    
    impl Default for ChannelDspState {
        fn default() -> Self {
            Self::new()
        }
    }

    impl ChannelDspState {
        pub fn set_gain_db(&mut self, db: f32) {
            self.target_gain_linear = 10.0_f32.powf(db / 20.0);
        }
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
                phase_invert: false,
                target_gain_linear: 1.0,
                target_reverb_send: 0.0,
                current_reverb_send: 0.0,
                current_gain_linear: 1.0,
                reverb: crate::audio::reverb::VirtualRoomReverb::new(48000.0),
                early_ref_buffer: vec![0.0; EARLY_REFLECTION_BUFFER_SIZE],
                early_ref_write_idx: 0,
                taps: [EarlyReflectionTapState::default(); crate::audio::acoustic::MAX_EARLY_REFLECTION_TAPS],
                target_early_ref_mix: 0.0,
                current_early_ref_mix: 0.0,
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
            
            // Smooth gain (Zipper noise prevention)
            if (self.current_gain_linear - self.target_gain_linear).abs() > 0.0001 {
                self.current_gain_linear += (self.target_gain_linear - self.current_gain_linear) * 0.002; // Simple one-pole smoothing
            } else {
                self.current_gain_linear = self.target_gain_linear;
            }
            
            out *= self.current_gain_linear;

            // Apply Phase Invert
            if self.phase_invert {
                out *= -1.0;
            }

            // Apply Early Reflections (Image-Source 1차 반사, 채널 고정 6탭)
            // Law 1: 사전 할당된 early_ref_buffer/taps만 사용, 힙 할당 없음.
            self.early_ref_buffer[self.early_ref_write_idx] = out;
            let mut er_sum = 0.0;
            for tap in self.taps.iter_mut() {
                // Law 3: target/current 1-pole 스무딩. current_delay_ms 스무딩과 동일 계수(0.005) 재사용.
                if (tap.target_delay_ms - tap.current_delay_ms).abs() > 0.001 {
                    tap.current_delay_ms += (tap.target_delay_ms - tap.current_delay_ms) * 0.005;
                } else {
                    tap.current_delay_ms = tap.target_delay_ms;
                }
                if (tap.target_gain - tap.current_gain).abs() > 0.0001 {
                    tap.current_gain += (tap.target_gain - tap.current_gain) * 0.005;
                } else {
                    tap.current_gain = tap.target_gain;
                }

                let delay_samples = (tap.current_delay_ms / 1000.0 * fs)
                    .clamp(0.0, (EARLY_REFLECTION_BUFFER_SIZE - 1) as f32) as usize;
                let read_idx = (self.early_ref_write_idx + EARLY_REFLECTION_BUFFER_SIZE - delay_samples)
                    % EARLY_REFLECTION_BUFFER_SIZE;
                er_sum += self.early_ref_buffer[read_idx] * tap.current_gain;
            }
            self.early_ref_write_idx = (self.early_ref_write_idx + 1) % EARLY_REFLECTION_BUFFER_SIZE;

            if (self.current_early_ref_mix - self.target_early_ref_mix).abs() > 0.0001 {
                self.current_early_ref_mix += (self.target_early_ref_mix - self.current_early_ref_mix) * 0.005;
            } else {
                self.current_early_ref_mix = self.target_early_ref_mix;
            }

            out += er_sum * self.current_early_ref_mix;

            // Apply Channel Independent Reverb
            if self.reverb.is_enabled && self.reverb.mix > 0.0 {
                out = self.reverb.process_mono(out);
            }

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

    #[cfg(test)]
    mod early_reflection_dsp_tests {
        use super::*;

        const FS: f32 = 48000.0;

        /// 목표 target/current 값을 즉시 스냅시켜 스무딩 수렴 대기 없이 정상상태를 만든다(테스트 전용).
        fn snap_early_ref(state: &mut ChannelDspState, tap_idx: usize, delay_ms: f32, gain: f32, mix: f32) {
            state.taps[tap_idx].target_delay_ms = delay_ms;
            state.taps[tap_idx].current_delay_ms = delay_ms;
            state.taps[tap_idx].target_gain = gain;
            state.taps[tap_idx].current_gain = gain;
            state.target_early_ref_mix = mix;
            state.current_early_ref_mix = mix;
        }

        /// B-4 완료 기준: 1kHz 사인파 입력 + 단일 tap(delay_ms=10, gain=0.5) 설정 시,
        /// 출력에 정확히 10ms(±1 샘플) 지연된 진폭 0.5의 사본이 합산되는지 오프라인 버퍼 처리로 검증.
        #[test]
        fn single_tap_produces_delayed_scaled_copy() {
            let mut state = ChannelDspState::new();
            snap_early_ref(&mut state, 0, 10.0, 0.5, 1.0);

            let delay_samples = (10.0 / 1000.0 * FS).round() as usize; // 480 samples @ 48kHz
            let total_len = delay_samples + 800;
            let freq = 1000.0_f32;

            let mut sine_in = vec![0.0f32; total_len];
            for (n, s) in sine_in.iter_mut().enumerate() {
                *s = (2.0 * std::f32::consts::PI * freq * n as f32 / FS).sin();
            }

            let mut output = vec![0.0f32; total_len];
            for n in 0..total_len {
                output[n] = state.process(sine_in[n], FS);
            }

            // 초반(딜레이 이전) 구간: tap이 아직 무음 이력을 읽으므로 dry 신호와 거의 동일해야 함.
            for n in 0..delay_samples.min(total_len) {
                assert!(
                    (output[n] - sine_in[n]).abs() < 1e-3,
                    "tap 도달 이전 샘플 {n}에서 dry 신호와 불일치: out={}, dry={}", output[n], sine_in[n]
                );
            }

            // 딜레이 이후: out[n] ≈ dry(n) + 0.5 * dry(n - delay_samples)
            // 허용오차 5e-3: DC 블로커(0.5Hz HPF)가 dry 경로에 미세한 위상/진폭 편차를 남기므로
            // bit-exact가 아닌 -46dB 상당의 근사치 검증으로 충분(딜레이/게인 자체의 정확성이 검증 목적).
            for n in delay_samples..total_len {
                let expected = sine_in[n] + 0.5 * sine_in[n - delay_samples];
                assert!(
                    (output[n] - expected).abs() < 5e-3,
                    "샘플 {n}에서 지연/게인 불일치: out={}, expected={}", output[n], expected
                );
            }
        }

        /// early_ref_mix=0일 때(디지털 무음 주입 관례) tap 파라미터가 무엇이든 출력이
        /// tap이 아예 없는 기준 상태와 완전히 동일(bit-exact)해야 한다 - Law 준수 회귀 방지.
        #[test]
        fn zero_mix_produces_bit_exact_output_regardless_of_tap_settings() {
            let mut baseline = ChannelDspState::new(); // taps/mix 전부 기본값(0)
            let mut with_taps = ChannelDspState::new();
            for i in 0..crate::audio::acoustic::MAX_EARLY_REFLECTION_TAPS {
                snap_early_ref(&mut with_taps, i, 5.0 + i as f32, 0.8, 0.0); // mix=0으로 고정
            }

            let total_len = 1000;
            for n in 0..total_len {
                let sample = (2.0 * std::f32::consts::PI * 1000.0 * n as f32 / FS).sin();
                let out_baseline = baseline.process(sample, FS);
                let out_with_taps = with_taps.process(sample, FS);
                assert_eq!(
                    out_baseline, out_with_taps,
                    "샘플 {n}에서 early_ref_mix=0인데도 출력이 달라짐(Law 회귀)"
                );
            }
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
