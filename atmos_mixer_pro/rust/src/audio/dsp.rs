pub mod dsp_utils {
    use crate::common::config::{EqBand, EqType};
    use crate::audio::svf::SvfFilter;
    
    pub const MAX_DSP_CHANNELS: usize = 128;
    pub const MAX_EQ_BANDS: usize = 8;

    /// EQ 계수를 다시 만드는 간격(샘플). 48kHz에서 약 1.3ms.
    pub const EQ_SMOOTH_INTERVAL: u32 = 64;
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
        /// 컷 필터 기울기(12/18/24dB/oct). 벨·쉘프·노치에서는 쓰지 않는다.
        pub slope: u8,
        /// 12/18/24 구성 출력의 섞임 비율. 슬로프가 바뀌면 20ms에 걸쳐 옮긴다.
        /// 세 구성을 늘 함께 돌려 두므로 전환 순간 필터 상태가 차갑지 않다
        /// (자동 EQ는 스피커가 벽에 가까워지면 드래그 도중 슬로프를 바꾼다).
        pub slope_w: [f32; 3],
        /// 18dB/oct = 1차 + 2차(Q 1.0), 24dB/oct = 2차(Q 0.541) + 2차(Q 1.307).
        /// Butterworth 연결이며 사용자 Q는 0.707 대비 비율로 공진만 조절한다.
        pub cut18_pole: crate::audio::svf::OnePole,
        pub cut18_svf: SvfFilter,
        pub cut24_a: SvfFilter,
        pub cut24_b: SvfFilter,
        /// 필터 적용 비율(0.0 = 우회, 1.0 = 완전 적용). 켜고 끌 때 이 값을
        /// 10ms에 걸쳐 미끄러뜨린다. 예전에는 즉시 우회/투입해서 +12dB 밴드를
        /// 끄는 순간 파형이 8배 넘게 튀었다(자동 EQ가 드래그 중 밴드를 켰다 껐다
        /// 하므로 틱 소리가 됐다).
        pub wet: f32,
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
                slope: 12,
                slope_w: [1.0, 0.0, 0.0],
                cut18_pole: crate::audio::svf::OnePole::new(),
                cut18_svf: SvfFilter::new(),
                cut24_a: SvfFilter::new(),
                cut24_b: SvfFilter::new(),
                wet: 0.0,
            }
        }
    
        pub fn update(&mut self, band: &EqBand, fs: f32) {
            let was_enabled = self.enabled;
            let type_changed = self.filter_type != band.filter_type;
            self.enabled = band.enabled;
            self.filter_type = band.filter_type;
            
            self.target_freq = band.freq.clamp(20.0, fs / 2.0 * 0.95);
            self.target_q = band.q_factor.clamp(0.1, 10.0);
            self.target_gain = band.gain;
            self.slope = match band.slope_db_per_oct {
                s if s >= 24 => 24,
                s if s >= 18 => 18,
                _ => 12,
            };
            
            if !was_enabled && self.enabled {
                self.current_freq = self.target_freq;
                self.current_gain = self.target_gain;
                self.current_q = self.target_q;
                self.slope_w = Self::slope_onehot(self.slope);
                self.recalculate(fs);
            } else if type_changed {
                // freq/gain/Q가 그대로면 process()의 스무딩 분기가 계수를 다시
                // 만들지 않는다. 예전에는 타입만 바꾸면 옛 타입으로 계속 걸렸다
                // (Bell -> HighShelf 후 12kHz 실측 0.01dB, 정상 +6dB).
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
            if matches!(self.filter_type, EqType::LowCut | EqType::HighCut) {
                let r = self.current_q / std::f32::consts::FRAC_1_SQRT_2;
                self.cut18_pole.update(&self.filter_type, fs, self.current_freq);
                self.cut18_svf.update_coefficients(&self.filter_type, fs, self.current_freq, 1.0 * r, 0.0);
                self.cut24_a.update_coefficients(&self.filter_type, fs, self.current_freq, 0.541_196_1 * r, 0.0);
                self.cut24_b.update_coefficients(&self.filter_type, fs, self.current_freq, 1.306_563 * r, 0.0);
            }
        }

        fn slope_onehot(slope: u8) -> [f32; 3] {
            match slope {
                24 => [0.0, 0.0, 1.0],
                18 => [0.0, 1.0, 0.0],
                _ => [1.0, 0.0, 0.0],
            }
        }
    
        #[inline(always)]
        pub fn process(&mut self, input: f32, fs: f32) -> f32 {
            let target_wet = if self.enabled { 1.0 } else { 0.0 };
            if self.wet <= 0.0 && target_wet <= 0.0 {
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

            let filtered = if matches!(self.filter_type, EqType::LowCut | EqType::HighCut) {
                let y12 = self.filter.process(input);
                let y18 = self.cut18_svf.process(self.cut18_pole.process(input));
                let y24 = self.cut24_b.process(self.cut24_a.process(input));
                let target = Self::slope_onehot(self.slope);
                let step = 1.0 / (0.02 * fs);
                for k in 0..3 {
                    let d = target[k] - self.slope_w[k];
                    self.slope_w[k] += d.clamp(-step, step);
                }
                y12 * self.slope_w[0] + y18 * self.slope_w[1] + y24 * self.slope_w[2]
            } else {
                self.filter.process(input)
            };
            let wet_step = 1.0 / (0.01 * fs);
            if self.wet < target_wet {
                self.wet = (self.wet + wet_step).min(target_wet);
            } else if self.wet > target_wet {
                self.wet = (self.wet - wet_step).max(target_wet);
            }
            input + (filtered - input) * self.wet
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
        /// 이 반사음의 고역 감쇠(4kHz 셸프 게인, dB). 실제 방처럼 반사음의 고역이
        /// 죽어서 돌아오게 한다(acoustic::EarlyReflectionTap::hf_shelf_db 참고).
        pub target_hf_shelf_db: f32,
        pub current_hf_shelf_db: f32,
    }

    #[derive(Clone)]
    pub struct ChannelDspState {
        /// EQ 계수 재계산까지 남은 샘플 카운터. `smooth_eq_step` 참고.
        pub eq_smooth_counter: u32,
        /// 현재 적용 중인 극성(-1.0 ~ +1.0 사이를 미끄러진다).
        ///
        /// `phase_invert`를 그냥 `out *= -1.0`으로 뒤집으면 파형 부호가 한 샘플
        /// 만에 반대가 되어 최대 진폭의 계단이 생긴다 — 확실하게 "딱" 하고
        /// 들린다. 스피커를 드래그해서 청취자 뒤쪽 경계를 넘나들면 이 뒤집기가
        /// 계속 일어난다(실기 보고: "스피커를 옮길 때마다 딱딱거린다").
        ///
        /// 신호와 그 반전은 완전히 역상이라 두 상태 사이를 지나가려면 잠깐
        /// 0을 통과할 수밖에 없다. 10ms에 걸쳐 미끄러지면 아주 짧은 딥으로
        /// 들리고, 계단처럼 튀지는 않는다.
        pub current_polarity: f32,
        /// 초기반사 탭별 크로스페이드 상태. `current_delay_ms`는 **고정된 읽기
        /// 위치**이고, 목표가 바뀌면 이전 위치(`tap_prev_delay_ms`)에서 새
        /// 위치로 교차 페이드한다. `tap_xfade`가 1.0이면 페이드 중이 아니다.
        ///
        /// 예전에는 탭 딜레이를 1-pole로 미끄러뜨려 읽기 위치가 연속으로
        /// 움직였다. 그건 테이프 속도를 바꾸는 것과 같아서, 스피커를 드래그해
        /// 반사 경로 길이가 바뀔 때마다 반사음 피치가 휘었다(실기 보고:
        /// "스피커를 움직이면 빨리감기 같은 소리"). 메인 딜레이는 이미
        /// 크로스페이드로 고쳤는데 이 탭들만 남아 있었다.
        pub tap_prev_delay_ms: [f32; crate::audio::acoustic::MAX_EARLY_REFLECTION_TAPS],
        pub tap_xfade: [f32; crate::audio::acoustic::MAX_EARLY_REFLECTION_TAPS],
        pub delay_buffer: Vec<f32>,
        pub delay_write_idx: usize,
        pub target_delay_ms: f32,
        pub current_delay_ms: f32,
        /// 딜레이 탭을 옮길 때 크로스페이드할 이전 탭 위치(ms).
        prev_delay_ms: f32,
        /// 크로스페이드 진행도(0.0~1.0). 1.0이면 진행 중이 아니다.
        delay_xfade: f32,

        pub target_bands: Vec<EqBand>,
        pub current_bands: Vec<EqBand>,
        pub eq_filters: Vec<EqFilterState>,
        pub dc_blocker: DcBlocker,

        // 공기 흡음은 채널 DSP에 두지 않는다. 채널 DSP 출력은 실제 스피커로 나가고, 현장에서는
        // 진짜 공기가 흡음한다(여기서도 걸면 두 번 깎인다). 헤드폰 미리듣기의 공기 흡음은
        // 바이노럴 전파 흉내에서 건다(binaural.rs Propagation).
        pub phase_invert: bool,
        pub target_gain_linear: f32,
        pub target_reverb_send: f32,
        pub current_reverb_send: f32,
        /// 공간 효과(초기반사·리버브)를 빼야 하는 채널인가.
        ///
        /// 베이스 매니지먼트로 서브우퍼가 된 채널이 여기에 해당한다. 서브 출력에는 메인에서
        /// 넘어온 저역이 섞이는데, 거기에 리버브·초기반사가 걸리면 저역이 웅웅거린다(표준
        /// 서브 출력 체인은 딜레이·EQ·레벨만 건다). 켜면 두 효과의 목표를 0으로 보고 기존
        /// 스무딩으로 서서히 뺀다 — 사용자가 설정한 값은 건드리지 않는다.
        pub mute_room_effects: bool,
        pub current_gain_linear: f32,
        pub reverb: crate::audio::reverb::VirtualRoomReverb,

        // 초기반사음(Image-Source 1차 반사) - 채널 고정 6슬롯, 사전 할당된 링버퍼만 사용(Law 1).
        pub early_ref_buffer: Vec<f32>,
        pub early_ref_write_idx: usize,
        pub taps: [EarlyReflectionTapState; crate::audio::acoustic::MAX_EARLY_REFLECTION_TAPS],
        /// 탭별 고역 셸프 필터. 계수는 EQ와 같은 주기(EQ_SMOOTH_INTERVAL)로만 갱신한다
        /// (매 샘플 tan() 재계산을 피하고, 값 변화도 그만큼 부드럽게 따라간다).
        pub tap_hf_shelf: [crate::audio::svf::SvfFilter;
            crate::audio::acoustic::MAX_EARLY_REFLECTION_TAPS],
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
                eq_smooth_counter: 0,
                current_polarity: 1.0,
                tap_prev_delay_ms: [0.0; crate::audio::acoustic::MAX_EARLY_REFLECTION_TAPS],
                tap_xfade: [1.0; crate::audio::acoustic::MAX_EARLY_REFLECTION_TAPS],
                delay_buffer: vec![0.0; DELAY_BUFFER_SIZE],
                delay_write_idx: 0,
                target_delay_ms: 0.0,
                current_delay_ms: 0.0,
                prev_delay_ms: 0.0,
                delay_xfade: 1.0,
                target_bands: vec![EqBand::default(); MAX_EQ_BANDS],
                current_bands: vec![EqBand::default(); MAX_EQ_BANDS],
                eq_filters,
                dc_blocker: DcBlocker::new(),
                phase_invert: false,
                target_gain_linear: 1.0,
                // 센드를 받기 전(엔진 기동 직후, 스피커가 없는 채널)에는 예전처럼
                // 랙 설정대로 리버브가 걸리도록 1.0(100%)에서 시작한다.
                target_reverb_send: 1.0,
                current_reverb_send: 1.0,
                mute_room_effects: false,
                current_gain_linear: 1.0,
                reverb: crate::audio::reverb::VirtualRoomReverb::new(48000.0),
                early_ref_buffer: vec![0.0; EARLY_REFLECTION_BUFFER_SIZE],
                early_ref_write_idx: 0,
                taps: [EarlyReflectionTapState::default(); crate::audio::acoustic::MAX_EARLY_REFLECTION_TAPS],
                // SvfFilter::new()는 m0=1 / m1=m2=0이라 계수 갱신 전에는 그대로 통과시킨다.
                tap_hf_shelf: std::array::from_fn(|_| crate::audio::svf::SvfFilter::new()),
                target_early_ref_mix: 0.0,
                current_early_ref_mix: 0.0,
            }
        }
    
        pub fn update_delay_target(&mut self, target_delay_ms: f32) {
            self.target_delay_ms = target_delay_ms.clamp(0.0, 1000.0);
        }

        /// 밴드 값이 **실제로 달라졌을 때만** 필터 계수를 다시 만든다.
        ///
        /// `eq_filters[i].update()`는 계수를 즉시 스냅한다(스무딩 없음). 예전에는
        /// 값이 그대로여도 호출할 때마다 무조건 갱신했는데, 스피커를 드래그하면
        /// `recalculate_spatial_dsp()`가 초당 30번 이 함수를 부르므로 계수가
        /// 계속 튀어 딸깍거리는 잡음이 났다(실기 보고: "스피커를 움직이면 소리가
        /// 끊기면서 나온다"). 오프액시스 EQ는 각도가 의미 있게 변할 때만
        /// 달라지므로, 같은 값이면 건너뛰는 것만으로 대부분의 잡음이 사라진다.
        ///
        /// 부수 효과로 오디오 스레드의 불필요한 계수 재계산(삼각함수 포함)도
        /// 함께 줄어든다.
        pub fn update_eq_targets(&mut self, target_bands: &[EqBand], fs: f32) {
            fn same(a: &EqBand, b: &EqBand) -> bool {
                a.enabled == b.enabled
                    && a.filter_type == b.filter_type
                    && (a.freq - b.freq).abs() < 1e-3
                    && (a.gain - b.gain).abs() < 1e-3
                    && (a.q_factor - b.q_factor).abs() < 1e-3
                    // 슬로프만 바뀐 밴드도 변화로 봐야 한다. 빠뜨리면 12 -> 24
                    // 변경이 "같은 밴드"로 판정돼 무시된다.
                    && a.slope_db_per_oct == b.slope_db_per_oct
            }

            let _ = fs; // 계수는 smooth_eq_block에서 만든다

            let limit = target_bands.len().min(MAX_EQ_BANDS);

            // **목표만** 갱신한다. 계수를 여기서 바로 갈아끼우면 바이쿼드가
            // 한 샘플 만에 바뀌어 딸깍 소리가 난다(Law 3 위반). 스피커를
            // 움직이면 바닥 반사 딥 주파수 같은 값이 매 프레임 바뀌므로
            // 예전에는 드래그 내내 계속 딸깍거렸다.
            for (i, band) in target_bands.iter().enumerate().take(limit) {
                if same(&self.target_bands[i], band) {
                    continue;
                }
                self.target_bands[i] = band.clone();
            }

            // Fill remaining filters with defaults if target_bands is smaller than MAX_EQ_BANDS
            let default_band = EqBand::default();
            for i in limit..MAX_EQ_BANDS {
                if same(&self.target_bands[i], &default_band) {
                    continue;
                }
                self.target_bands[i] = default_band.clone();
            }
        }

        /// EQ 파라미터를 목표 쪽으로 한 스텝 끌어당기고, 움직인 밴드만
        /// 계수를 다시 만든다(Law 3: 파라미터를 스냅하지 않는다).
        ///
        /// [EQ_SMOOTH_INTERVAL] 샘플마다 `process`가 직접 호출한다.
        ///
        /// 블록(1024샘플)마다 한 번으로는 부족하다. 18dB짜리 변경이면 한
        /// 블록 만에 9dB가 움직여서 여전히 계단이 지고 딸깍 소리가 난다
        /// (test_eq_smoothing_no_click이 실제로 그걸 잡아냈다). 64샘플
        /// (1.3ms)마다 조금씩 움직이면 파형이 끊기지 않으면서도 60ms 안에
        /// 목표에 도달한다.
        ///
        /// 샘플마다 하지 않는 이유는 계수 계산에 sin/cos/pow가 들어가기
        /// 때문이다. 64샘플 간격이면 비용이 1/64로 떨어진다. 힙 할당은 없다.
        pub fn smooth_eq_step(&mut self, fs: f32) {
            // 스텝당 5%. 64샘플 간격 기준으로 90% 도달에 약 60ms.
            const COEF: f32 = 0.05;
            const EPS_F: f32 = 1e-3;

            for i in 0..MAX_EQ_BANDS {
                let (t_enabled, t_type, t_freq, t_gain, t_q, t_slope) = {
                    let t = &self.target_bands[i];
                    (t.enabled, t.filter_type, t.freq, t.gain, t.q_factor, t.slope_db_per_oct)
                };

                let structural = self.current_bands[i].enabled != t_enabled
                    || self.current_bands[i].filter_type != t_type
                    || self.current_bands[i].slope_db_per_oct != t_slope;
                let df = t_freq - self.current_bands[i].freq;
                let dg = t_gain - self.current_bands[i].gain;
                let dq = t_q - self.current_bands[i].q_factor;

                if !structural
                    && df.abs() < EPS_F
                    && dg.abs() < EPS_F
                    && dq.abs() < EPS_F
                {
                    continue;
                }

                {
                    let c = &mut self.current_bands[i];
                    // 켜짐/타입은 사람이 명시적으로 바꾸는 값이고 중간 상태가
                    // 없으므로 그대로 반영한다.
                    c.enabled = t_enabled;
                    c.filter_type = t_type;
                    // 슬로프도 필터로 넘겨야 한다(빠뜨리면 늘 12dB/oct로 걸린다).
                    c.slope_db_per_oct = t_slope;

                    // 주파수는 기하(로그) 보간이라야 옥타브가 고르게 움직인다.
                    if df.abs() < EPS_F || c.freq <= 0.0 || t_freq <= 0.0 {
                        c.freq = t_freq;
                    } else {
                        c.freq *= (t_freq / c.freq).powf(COEF);
                        if (t_freq - c.freq).abs() < EPS_F {
                            c.freq = t_freq;
                        }
                    }

                    c.gain += dg * COEF;
                    if (t_gain - c.gain).abs() < EPS_F {
                        c.gain = t_gain;
                    }
                    c.q_factor += dq * COEF;
                    if (t_q - c.q_factor).abs() < EPS_F {
                        c.q_factor = t_q;
                    }
                }

                // current_bands와 eq_filters를 동시에 빌리지 않도록 분리한다.
                let (bands, filters) = (&self.current_bands, &mut self.eq_filters);
                filters[i].update(&bands[i], fs);
            }
        }
    
        /// 탭별 고역 셸프를 목표값으로 조금씩 옮긴다([EQ_SMOOTH_INTERVAL] 샘플마다 호출).
        ///
        /// 스피커를 드래그하면 반사 각도가 계속 바뀌어 이 값도 따라 움직인다. 계수를 즉시
        /// 갈아끼우면 딸깍 소리가 나므로(Law 3) 조금씩 옮기고, 변화가 없으면 건드리지 않는다.
        fn smooth_tap_hf_shelf_step(&mut self, fs: f32) {
            for i in 0..crate::audio::acoustic::MAX_EARLY_REFLECTION_TAPS {
                let target = self.taps[i].target_hf_shelf_db;
                let current = self.taps[i].current_hf_shelf_db;
                let diff = target - current;
                if diff.abs() < 0.01 {
                    if current != target {
                        self.taps[i].current_hf_shelf_db = target;
                        self.tap_hf_shelf[i].update_coefficients(
                            &crate::common::config::EqType::HighShelf,
                            fs,
                            crate::audio::acoustic::EARLY_REFLECTION_HF_SHELF_HZ,
                            0.707,
                            target,
                        );
                    }
                    continue;
                }
                // 약 40ms(=EQ와 같은 감각)로 따라간다.
                let next = current + diff * 0.15;
                self.taps[i].current_hf_shelf_db = next;
                self.tap_hf_shelf[i].update_coefficients(
                    &crate::common::config::EqType::HighShelf,
                    fs,
                    crate::audio::acoustic::EARLY_REFLECTION_HF_SHELF_HZ,
                    0.707,
                    next,
                );
            }
        }

        #[inline(always)]
        pub fn process(&mut self, input: f32, fs: f32) -> f32 {
            // EQ 계수는 64샘플마다 조금씩 옮긴다(위 smooth_eq_step 주석 참고).
            self.eq_smooth_counter += 1;
            if self.eq_smooth_counter >= EQ_SMOOTH_INTERVAL {
                self.eq_smooth_counter = 0;
                self.smooth_eq_step(fs);
                self.smooth_tap_hf_shelf_step(fs);
            }

            // 딜레이 탭 이동은 **크로스페이드**로 처리한다(예전에는 탭을 서서히
            // 끌고 갔다).
            //
            // 딜레이 라인의 읽기 위치를 연속적으로 움직이면 재생 속도가 바뀌는
            // 것과 같아서 피치가 휜다. 움직이는 음원(도플러)에는 맞지만, 스피커
            // 위치를 다시 잡는 건 정지한 스피커의 시간정렬 보정값을 바꾸는
            // 것이므로 피치가 휘면 안 된다. 실기 보고: "스피커를 움직이면
            // 테이프 스톱/스타트 같은 소리가 난다."
            //
            // 이제 목표가 의미 있게 바뀌면 이전 탭과 새 탭에서 동시에 읽어
            // 짧게(20ms) 교차 페이드한다. 읽기 위치 자체는 각각 고정이므로
            // 피치 변화가 없다.
            const XFADE_MS: f32 = 20.0;
            let diff = self.target_delay_ms - self.current_delay_ms;
            if diff.abs() > 0.01 {
                // 이미 페이드 중이면 현재 들리는 지점을 새 출발점으로 삼는다.
                self.prev_delay_ms = if self.delay_xfade < 1.0 {
                    self.prev_delay_ms * (1.0 - self.delay_xfade)
                        + self.current_delay_ms * self.delay_xfade
                } else {
                    self.current_delay_ms
                };
                self.current_delay_ms = self.target_delay_ms;
                self.delay_xfade = 0.0;
            }

            // Write to delay buffer
            self.delay_buffer[self.delay_write_idx] = input;

            // 주어진 지연(ms)에서 hermite 보간으로 한 샘플 읽는다.
            let read_at = |buf: &[f32], write_idx: usize, delay_ms: f32| -> f32 {
                let delay_samples =
                    (delay_ms / 1000.0 * fs).clamp(0.0, (DELAY_BUFFER_SIZE - 4) as f32);
                let delay_int = delay_samples.floor() as usize;
                let delay_frac = delay_samples - delay_int as f32;

                let i0 = (write_idx + DELAY_BUFFER_SIZE - delay_int + 1) % DELAY_BUFFER_SIZE;
                let i1 = (write_idx + DELAY_BUFFER_SIZE - delay_int) % DELAY_BUFFER_SIZE;
                let i2 = (write_idx + DELAY_BUFFER_SIZE - delay_int - 1) % DELAY_BUFFER_SIZE;
                let i3 = (write_idx + DELAY_BUFFER_SIZE - delay_int - 2) % DELAY_BUFFER_SIZE;

                interpolate_hermite(buf[i0], buf[i1], buf[i2], buf[i3], delay_frac)
            };

            let mut out = if self.delay_xfade < 1.0 {
                let a = read_at(&self.delay_buffer, self.delay_write_idx, self.prev_delay_ms);
                let b = read_at(&self.delay_buffer, self.delay_write_idx, self.current_delay_ms);
                // 등출력 교차 페이드(상관 없는 두 탭이 겹칠 때 음량 꺼짐 방지).
                let t = self.delay_xfade;
                let g_a = ((1.0 - t) * std::f32::consts::FRAC_PI_2).sin();
                let g_b = (t * std::f32::consts::FRAC_PI_2).sin();
                self.delay_xfade = (self.delay_xfade + 1000.0 / (XFADE_MS * fs)).min(1.0);
                a * g_a + b * g_b
            } else {
                read_at(&self.delay_buffer, self.delay_write_idx, self.current_delay_ms)
            };
            prevent_denormal(&mut out);
            
            self.delay_write_idx = (self.delay_write_idx + 1) % DELAY_BUFFER_SIZE;
            
            // Apply EQs
            for filter in self.eq_filters.iter_mut() {
                out = filter.process(out, fs);
            }
            
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
            // 극성은 목표값으로 미끄러진다(위 current_polarity 주석 참고).
            {
                let target_polarity = if self.phase_invert { -1.0 } else { 1.0 };
                let dp = target_polarity - self.current_polarity;
                if dp.abs() > 1e-6 {
                    // 10ms에 0 -> 1 전체를 건너가는 속도. 부호 전환은 2.0만큼
                    // 움직여야 하므로 실제로는 약 20ms가 걸린다.
                    let step = 1000.0 / (10.0 * fs);
                    self.current_polarity += dp.signum() * step.min(dp.abs());
                } else {
                    self.current_polarity = target_polarity;
                }
                out *= self.current_polarity;
            }

            // Apply Early Reflections (Image-Source 1차 반사, 채널 고정 6탭)
            // Law 1: 사전 할당된 early_ref_buffer/taps만 사용, 힙 할당 없음.
            self.early_ref_buffer[self.early_ref_write_idx] = out;
            let mut er_sum = 0.0;
            // 30ms 교차 페이드. 페이드 도중 들어온 새 목표는 기다렸다가, 끝나는
            // 즉시 가장 최근 목표로 다음 페이드를 시작한다(중간값은 건너뛴다).
            let tap_xfade_step = 1000.0 / (30.0 * fs);
            for (ti, tap) in self.taps.iter_mut().enumerate() {
                if self.tap_xfade[ti] >= 1.0
                    && (tap.target_delay_ms - tap.current_delay_ms).abs() > 0.001
                {
                    self.tap_prev_delay_ms[ti] = tap.current_delay_ms;
                    tap.current_delay_ms = tap.target_delay_ms;
                    self.tap_xfade[ti] = 0.0;
                }
                if (tap.target_gain - tap.current_gain).abs() > 0.0001 {
                    tap.current_gain += (tap.target_gain - tap.current_gain) * 0.005;
                } else {
                    tap.current_gain = tap.target_gain;
                }

                // 고정 위치에서만 읽는다(읽기 위치가 움직이지 않으므로 피치 불변).
                let write_idx = self.early_ref_write_idx;
                let read_at = |buf: &[f32], delay_ms: f32| -> f32 {
                    let delay_samples = (delay_ms / 1000.0 * fs)
                        .clamp(0.0, (EARLY_REFLECTION_BUFFER_SIZE - 1) as f32) as usize;
                    buf[(write_idx + EARLY_REFLECTION_BUFFER_SIZE - delay_samples)
                        % EARLY_REFLECTION_BUFFER_SIZE]
                };
                let reflected = if self.tap_xfade[ti] < 1.0 {
                    let t = self.tap_xfade[ti];
                    let old_tap = read_at(&self.early_ref_buffer, self.tap_prev_delay_ms[ti]);
                    let new_tap = read_at(&self.early_ref_buffer, tap.current_delay_ms);
                    self.tap_xfade[ti] = (t + tap_xfade_step).min(1.0);
                    old_tap * ((1.0 - t) * std::f32::consts::FRAC_PI_2).sin()
                        + new_tap * (t * std::f32::consts::FRAC_PI_2).sin()
                } else {
                    read_at(&self.early_ref_buffer, tap.current_delay_ms)
                };
                er_sum += self.tap_hf_shelf[ti].process(reflected) * tap.current_gain;
            }
            self.early_ref_write_idx = (self.early_ref_write_idx + 1) % EARLY_REFLECTION_BUFFER_SIZE;

            let er_target = if self.mute_room_effects { 0.0 } else { self.target_early_ref_mix };
            if (self.current_early_ref_mix - er_target).abs() > 0.0001 {
                self.current_early_ref_mix += (er_target - self.current_early_ref_mix) * 0.005;
            } else {
                self.current_early_ref_mix = er_target;
            }

            out += er_sum * self.current_early_ref_mix;

            // 채널 리버브 센드(스피커 인스펙터의 Reverb Send). 초기반사 믹스와
            // 같은 1-pole 스무딩으로 따라간다(DSP Law 3: 순간 전환 금지).
            let send_target = if self.mute_room_effects { 0.0 } else { self.target_reverb_send };
            if (self.current_reverb_send - send_target).abs() > 0.0001 {
                self.current_reverb_send += (send_target - self.current_reverb_send) * 0.005;
            } else {
                self.current_reverb_send = send_target;
            }

            // Apply Channel Independent Reverb
            //
            // 출력 = 원음 + (리버브 통과음 - 원음) x 센드
            //   센드 0 -> 원음 그대로(잔향 없음), 센드 1 -> 랙 설정 그대로.
            // 즉 실제로 걸리는 잔향 양은 랙 MIX x 센드다.
            //
            // 예전에는 SetChannelReverbSend가 target_reverb_send에 값을 쓰기만
            // 하고 여기서 한 번도 읽지 않아, 센드를 바꿔도 소리가 변하지 않았다.
            //
            // 리버브 네트워크에는 센드와 무관하게 항상 신호를 넣는다. 센드가 0일
            // 때 처리를 건너뛰면 네트워크에 오래된 잔향이 얼어붙어 있다가 센드를
            // 올리는 순간 튀어나온다. 처리 조건은 예전과 같으므로 CPU 부하도 같다.
            if self.reverb.is_enabled && self.reverb.mix > 0.0 {
                let reverbed = self.reverb.process_mono(out);
                out += (reverbed - out) * self.current_reverb_send;
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
                // 셸프 감쇠량도 스무딩된 차단 주파수에서 구한다(Law 3). 목표 거리에서 바로 구하면
                // 헤드폰 경로에서 스피커를 끄는 동안 새 거리가 들어올 때마다 감쇠량이 계단처럼 뛴다.
                let smoothed_dist = (max_cutoff - self.current_cutoff) / (max_cutoff - min_cutoff);
                self.lpf.update_coefficients(&EqType::HighShelf, sample_rate, self.current_cutoff, 0.707, -smoothed_dist * 24.0);
            }
        }

        pub fn process(&mut self, input: f32) -> f32 {
            self.lpf.process(input)
        }
    }
}
