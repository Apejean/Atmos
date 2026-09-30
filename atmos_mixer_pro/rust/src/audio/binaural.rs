use realfft::RealFftPlanner;
use rustfft::num_complex::Complex;
use sofa_reader::{SofaFile, SourcePosition};

pub struct BinauralChannel {
    ir_left_freq: Vec<Complex<f32>>,
    ir_right_freq: Vec<Complex<f32>>,
    target_ir_left_freq: Vec<Complex<f32>>,
    target_ir_right_freq: Vec<Complex<f32>>,
    fft_size: usize,
    input_buffer: Vec<f32>,
    /// 현재 IR로 계산한 오버랩 테일(다음 블록 앞부분에 더해진다).
    overlap_add_left: Vec<f32>,
    overlap_add_right: Vec<f32>,
    /// 목표 IR로 계산한 오버랩 테일. 전환 중에는 테일을 IR별로 따로 들고
    /// 있다가 다음 블록에서 **샘플마다** 같은 섞임 비율로 섞는다.
    ///
    /// 예전에는 테일을 블록 끝 시점 비율 하나로 미리 섞어 저장했다. 다음 블록
    /// 앞부분은 샘플마다 비율이 달라지는데 테일만 고정이라, 직전 입력의 응답이
    /// 몰려 있는 HRIR 피크 지점에서 어긋나 틱 소리가 났다.
    overlap_target_left: Vec<f32>,
    overlap_target_right: Vec<f32>,
    /// 직전 process_block의 블록 길이. 전환 시작 시 새 IR의 테일을 직전 입력
    /// 스펙트럼으로 다시 만들 때 테일이 어디서 시작하는지 알아야 한다.
    last_block_size: usize,
    r2c: std::sync::Arc<dyn realfft::RealToComplex<f32>>,
    c2r: std::sync::Arc<dyn realfft::ComplexToReal<f32>>,
    input_freq: Vec<Complex<f32>>,
    out_left_freq: Vec<Complex<f32>>,
    out_right_freq: Vec<Complex<f32>>,
    out_left_time: Vec<f32>,
    out_right_time: Vec<f32>,
    out_left_time_target: Vec<f32>,
    out_right_time_target: Vec<f32>,
    crossfade_phase: f32,
    is_switching: bool,

    // update_hrtf()에서 재사용하는 임시 패딩 버퍼. 오디오 콜백(process_interleaved)에서
    // 호출될 수 있으므로 매번 vec![]로 새로 할당하지 않고 사전 할당된 버퍼를 덮어쓴다(Law1 준수).
    scratch_pad_left: Vec<f32>,
    scratch_pad_right: Vec<f32>,
}

impl BinauralChannel {
    pub fn new(ir_left: &[f32], ir_right: &[f32], block_size: usize) -> Self {
        let ir_len = ir_left.len().max(ir_right.len());
        let fft_size = (block_size + ir_len - 1).next_power_of_two();
        
        let mut planner = RealFftPlanner::<f32>::new();
        let r2c = planner.plan_fft_forward(fft_size);
        let c2r = planner.plan_fft_inverse(fft_size);
        
        let mut ir_left_padded = vec![0.0; fft_size];
        ir_left_padded[..ir_left.len()].copy_from_slice(ir_left);
        let mut ir_left_freq = r2c.make_output_vec();
        let _ = r2c.process(&mut ir_left_padded, &mut ir_left_freq);

        let mut ir_right_padded = vec![0.0; fft_size];
        ir_right_padded[..ir_right.len()].copy_from_slice(ir_right);
        let mut ir_right_freq = r2c.make_output_vec();
        let _ = r2c.process(&mut ir_right_padded, &mut ir_right_freq);

        let target_ir_left_freq = ir_left_freq.clone();
        let target_ir_right_freq = ir_right_freq.clone();

        let input_freq = vec![Complex::new(0.0, 0.0); ir_left_freq.len()];
        let out_left_freq = vec![Complex::new(0.0, 0.0); ir_left_freq.len()];
        let out_right_freq = vec![Complex::new(0.0, 0.0); ir_left_freq.len()];
        
        let out_left_time = vec![0.0; fft_size];
        let out_right_time = vec![0.0; fft_size];
        let out_left_time_target = vec![0.0; fft_size];
        let out_right_time_target = vec![0.0; fft_size];

        Self {
            ir_left_freq,
            ir_right_freq,
            target_ir_left_freq,
            target_ir_right_freq,
            fft_size,
            input_buffer: vec![0.0; fft_size],
            overlap_add_left: vec![0.0; fft_size],
            overlap_add_right: vec![0.0; fft_size],
            overlap_target_left: vec![0.0; fft_size],
            overlap_target_right: vec![0.0; fft_size],
            last_block_size: block_size,
            r2c,
            c2r,
            input_freq,
            out_left_freq,
            out_right_freq,
            out_left_time,
            out_right_time,
            out_left_time_target,
            out_right_time_target,
            crossfade_phase: 1.0,
            is_switching: false,

            scratch_pad_left: vec![0.0; fft_size],
            scratch_pad_right: vec![0.0; fft_size],
        }
    }

    pub fn update_hrtf(&mut self, new_ir_left: &[f32], new_ir_right: &[f32]) {
        // 아직 이전 전환이 진행 중이면, **지금 들리고 있는 중간 상태**를
        // 새 출발점으로 삼는다.
        //
        // 예전에는 target만 갈아끼우고 phase를 0으로 되돌렸다. 그러면 현재
        // IR은 여전히 두 단계 전의 것이라, 출력이 방금까지 가던 방향에서
        // 뒤로 튕겨 나갔다가 다시 새 목표로 향한다. 스피커를 드래그하면 이
        // 튕김이 계속 반복되는데, HRIR에는 ITD(좌우 도달 시간차)가 들어
        // 있으므로 그 왕복이 곧 앞뒤로 흔들리는 딜레이가 되어 피치가 휜다
        // (실기 보고: "빨리감기 같은 소리").
        if self.is_switching && self.crossfade_phase > 0.0 {
            let t = self.crossfade_phase.min(1.0);
            for i in 0..self.ir_left_freq.len() {
                self.ir_left_freq[i] =
                    self.ir_left_freq[i] * (1.0 - t) + self.target_ir_left_freq[i] * t;
                self.ir_right_freq[i] =
                    self.ir_right_freq[i] * (1.0 - t) + self.target_ir_right_freq[i] * t;
            }
            // 테일도 같은 비율로 접어야 IR과 테일이 서로 맞는다.
            for i in 0..self.fft_size {
                self.overlap_add_left[i] =
                    self.overlap_add_left[i] * (1.0 - t) + self.overlap_target_left[i] * t;
                self.overlap_add_right[i] =
                    self.overlap_add_right[i] * (1.0 - t) + self.overlap_target_right[i] * t;
            }
        }

        let len_l = new_ir_left.len().min(self.fft_size);
        self.scratch_pad_left.fill(0.0);
        self.scratch_pad_left[..len_l].copy_from_slice(&new_ir_left[..len_l]);
        let _ = self.r2c.process(&mut self.scratch_pad_left, &mut self.target_ir_left_freq);

        let len_r = new_ir_right.len().min(self.fft_size);
        self.scratch_pad_right.fill(0.0);
        self.scratch_pad_right[..len_r].copy_from_slice(&new_ir_right[..len_r]);
        let _ = self.r2c.process(&mut self.scratch_pad_right, &mut self.target_ir_right_freq);

        // 새 목표 IR에 대한 "직전 블록 입력의 테일"을 정확히 만든다. 이 시점의
        // input_freq에는 아직 직전 블록의 입력 스펙트럼이 남아 있다(다음
        // process_block이 덮어쓰기 전). 이게 없으면 전환 첫 블록 앞부분이 옛
        // IR 테일만 들고 시작해 어긋난다.
        let scale = 1.0 / self.fft_size as f32;
        for i in 0..self.input_freq.len() {
            self.out_left_freq[i] = self.input_freq[i] * self.target_ir_left_freq[i];
            self.out_right_freq[i] = self.input_freq[i] * self.target_ir_right_freq[i];
        }
        let _ = self.c2r.process(&mut self.out_left_freq, &mut self.out_left_time_target);
        let _ = self.c2r.process(&mut self.out_right_freq, &mut self.out_right_time_target);
        self.overlap_target_left.fill(0.0);
        self.overlap_target_right.fill(0.0);
        let last = self.last_block_size.min(self.fft_size);
        for i in last..self.fft_size {
            self.overlap_target_left[i - last] = self.out_left_time_target[i] * scale;
            self.overlap_target_right[i - last] = self.out_right_time_target[i] * scale;
        }

        self.is_switching = true;
        self.crossfade_phase = 0.0;
    }

        /// `input`을 이 채널의 현재 HRIR로 컨볼브해 `out_left`/`out_right`에
        /// **누적(가산)**한다 — 대입이 아니다. 여러 채널을 한 스테레오 버스로
        /// 합산해야 하는 호출부(`process_interleaved`)가 루프 진입 전
        /// `mix_left`/`mix_right`를 0으로 채워두고 채널마다 이 함수를 호출하는
        /// 구조이기 때문이다. 예전에는 여기서 `=`로 대입해서, 채널이
        /// 2개 이상일 때 나중 채널(무음이어도)이 앞선 채널의 값을 통째로
        /// 지워버렸다 — 항상 채널 1개로만 테스트되거나 전 채널이 무음이라
        /// 드러나지 않았던 잠재 결함이었다.
        pub fn process_block(&mut self, input: &[f32], out_left: &mut [f32], out_right: &mut [f32]) {
        let block_size = input.len().min(self.fft_size);
        self.last_block_size = block_size;
        // 이번 블록이 전환 중에 시작했는지. 페이드가 블록 도중에 끝나면
        // is_switching이 먼저 false가 되는데, 오버랩 테일은 여전히 이 블록에서
        // 계산한 새 IR 결과(out_*_time_target)로 채워야 한다.
        let was_switching = self.is_switching;
        self.input_buffer.fill(0.0);
        self.input_buffer[..block_size].copy_from_slice(&input[..block_size]);

        let _ = self.r2c.process(&mut self.input_buffer, &mut self.input_freq);

        // Convolve with current IR
        for i in 0..self.input_freq.len() {
            self.out_left_freq[i] = self.input_freq[i] * self.ir_left_freq[i];
            self.out_right_freq[i] = self.input_freq[i] * self.ir_right_freq[i];
        }
        let _ = self.c2r.process(&mut self.out_left_freq, &mut self.out_left_time);
        let _ = self.c2r.process(&mut self.out_right_freq, &mut self.out_right_time);

        let scale = 1.0 / self.fft_size as f32;
        
        // If switching, convolve with target IR and crossfade
        if self.is_switching {
            for i in 0..self.input_freq.len() {
                self.out_left_freq[i] = self.input_freq[i] * self.target_ir_left_freq[i];
                self.out_right_freq[i] = self.input_freq[i] * self.target_ir_right_freq[i];
            }
            let _ = self.c2r.process(&mut self.out_left_freq, &mut self.out_left_time_target);
            let _ = self.c2r.process(&mut self.out_right_freq, &mut self.out_right_time_target);
            
            // 한 블록(약 21ms) 안에 끝내면 ITD가 그만큼 급하게 움직여
            // 피치가 휜다. 여러 블록에 걸쳐 천천히 건너간다.
            const XFADE_BLOCKS: f32 = 4.0;
            let fade_step = 1.0 / (block_size as f32 * XFADE_BLOCKS);
            
            for i in 0..block_size {
                self.crossfade_phase += fade_step;
                if self.crossfade_phase >= 1.0 {
                    self.crossfade_phase = 1.0;
                    self.is_switching = false;
                    self.ir_left_freq.copy_from_slice(&self.target_ir_left_freq);
                    self.ir_right_freq.copy_from_slice(&self.target_ir_right_freq);
                }
                
                let cur_l = self.out_left_time[i] * scale;
                let cur_r = self.out_right_time[i] * scale;
                let tar_l = self.out_left_time_target[i] * scale;
                let tar_r = self.out_right_time_target[i] * scale;
                
                let mix_l = cur_l * (1.0 - self.crossfade_phase) + tar_l * self.crossfade_phase;
                let mix_r = cur_r * (1.0 - self.crossfade_phase) + tar_r * self.crossfade_phase;
                
                let p = self.crossfade_phase;
                out_left[i] += mix_l
                    + self.overlap_add_left[i] * (1.0 - p)
                    + self.overlap_target_left[i] * p;
                out_right[i] += mix_r
                    + self.overlap_add_right[i] * (1.0 - p)
                    + self.overlap_target_right[i] * p;
            }
        } else {
            for i in 0..block_size {
                out_left[i] += self.out_left_time[i] * scale + self.overlap_add_left[i];
                out_right[i] += self.out_right_time[i] * scale + self.overlap_add_right[i];
            }
        }
        
        // 오버랩 테일을 IR별로 따로 저장한다(overlap_target_* 필드 주석 참고).
        self.overlap_add_left.fill(0.0);
        self.overlap_add_right.fill(0.0);
        if was_switching {
            self.overlap_target_left.fill(0.0);
            self.overlap_target_right.fill(0.0);
        }
        for i in block_size..self.fft_size {
            let dst_i = i - block_size;
            self.overlap_add_left[dst_i] = self.out_left_time[i] * scale;
            self.overlap_add_right[dst_i] = self.out_right_time[i] * scale;
            if was_switching {
                self.overlap_target_left[dst_i] = self.out_left_time_target[i] * scale;
                self.overlap_target_right[dst_i] = self.out_right_time_target[i] * scale;
            }
        }
        // 페이드가 이번 블록에서 끝났으면 현재 IR이 곧 목표 IR이므로 테일도 그쪽.
        if was_switching && !self.is_switching {
            self.overlap_add_left.copy_from_slice(&self.overlap_target_left);
            self.overlap_add_right.copy_from_slice(&self.overlap_target_right);
        }
    }
}

pub struct VirtualMixRoomBinaural {
    channels: Vec<BinauralChannel>,
    pub enabled: bool,
    temp_channel_buffers: Vec<Vec<f32>>,
    mix_left: Vec<f32>,
    mix_right: Vec<f32>,
    current_yaw: f32,
    current_pitch: f32,
    current_roll: f32,

    // MIT KEMAR SOFA HRTF 데이터셋. new()에서 1회만 로드하며, 오디오 콜백에서는
    // 이 데이터를 읽기만 한다(신규 할당/파일 I/O 절대 금지, Law1 준수).
    hrtf_db: Option<SofaFile>,
    // 데이터셋 측정 거리(m). 방위각 보간 시 타깃 위치의 반경 성분으로 사용.
    nominal_distance: f32,
    // 방위각 보간 결과를 저장하는 사전 할당 스크래치 버퍼(Law1 준수).
    interp_ir_left: Vec<f32>,
    interp_ir_right: Vec<f32>,

    // SOFA 측정치(M≈710개)를 고도 링(같은 고도, 방위각 순 정렬)으로 미리 묶은 공간 인덱스.
    // new()에서 1회만 구축하며, 오디오 콜백은 이 인덱스를 읽기만 하므로 매 버퍼마다
    // 전체 측정치를 선형 스캔하지 않는다(Law1/예측 가능한 CPU 사용량 준수).
    // 예전에는 방위각 버킷만 있어서 고도를 볼 수 없었다(모든 스피커를 귀 높이로 렌더링).
    elevation_rings: Vec<ElevationRing>,
    /// 데이터셋이 가진 고도 범위(도). 이 밖의 스피커는 가장 가까운 끝 고도로 본다.
    elevation_range: (f32, f32),

    /// 채널별 기준 방위각(도, 0°=정면/+Y, +90°=오른쪽/+X). 리스너 기준
    /// 위치에서 계산되며, 헤드 요각과 합쳐져 최종 조회 방위각이 된다.
    /// `docs/02_Planning_and_Specs/BINAURAL_SPATIAL_RENDERING_SPEC.md` §2.1.
    /// 미바인딩 채널은 0.0(정면 취급)으로 안전하게 초기화된다.
    /// 헤드폰으로 렌더링할 채널 마스크(현재 보고 있는 방의 스피커만 true).
    /// 방을 지정하지 않으면 전부 true다.
    channel_enabled: Vec<bool>,
    channel_base_azimuth: Vec<f32>,
    /// 채널별 고도각(도, 청취 지점 귀 높이 기준 +위/−아래). 천장 가까이 매단 스피커가
    /// 위에서 들리게 한다. 좌표가 없으면 0(귀 높이).
    channel_base_elevation: Vec<f32>,
    /// 각 채널에 실제로 적용된 고도각(도). `applied_azimuth`와 같은 이유로 둔다.
    applied_elevation: Vec<f32>,
    /// 각 채널에 **실제로 적용된** 최종 방위각(도). 목표와 비교해서, 의미
    /// 있게 달라진 채널만 HRIR을 다시 만든다.
    ///
    /// 예전에는 위치 갱신이 한 번만 와도 `base_azimuth_dirty`가 서면서 **전
    /// 채널**의 최근접 3점 탐색 + IR 보간 + FFT를 콜백마다 다시 돌렸다.
    /// 스피커를 드래그하면 위치 갱신이 초당 30번씩 오므로 오디오 스레드가
    /// 그 작업에 잠겨 소리가 끊겼다(실기 보고: "스피커를 움직이는 순간
    /// 끊기면서 이상하게 들린다"). 실제로 각도가 바뀌는 건 드래그 중인 그
    /// 채널 하나뿐이다.
    applied_azimuth: Vec<f32>,
    /// 테스트용 HRIR 재생성 횟수 누적값. 오디오 스레드에서는 단순 증가만
    /// 하므로 비용이 없다.
    hrir_rebuild_count: u64,

    /// `channel_base_azimuth`가 갱신되어 이번 콜백에서 재보간이 필요함을
    /// 표시한다. 헤드 방향이 안 바뀌어도(예: 사용자가 스피커 레이아웃을
    /// 편집만 한 경우) 채널 위치가 바뀌면 다시 계산해야 하므로, 기존 요각
    /// 변화 감지와는 별도의 트리거다.
    base_azimuth_dirty: bool,
    /// 실제로 쓰고 있는 HRTF의 샘플레이트(로드 실패 시 None).
    hrtf_sample_rate: Option<f32>,

    /// 채널별 전파 흉내(스피커 → 청취 지점: 거리만큼 늦게, 거리에 반비례해 작게).
    ///
    /// 현장에서는 실제 공기가 이 지연·감쇠를 만들고, 스피커 보정(시간 정렬 딜레이·거리
    /// 게인)이 그걸 상쇄한다. 헤드폰에서도 같은 걸 흉내 내야 보정이 현장처럼 상쇄된다.
    /// 예전에는 모든 스피커를 같은 거리에 둔 것처럼 렌더링해서 보정만 남았다(가까운
    /// 서브가 23ms 늦게·작게 들려 크로스오버 부근이 지워졌다).
    propagation: Vec<Propagation>,
    /// 채널별 전파 지연 링 버퍼(new()에서 사전 할당, Law 1). 쓰기 위치는 공통이다.
    propagation_buffers: Vec<Vec<f32>>,
    propagation_write_idx: usize,
    propagation_len: usize,
    /// 켠 뒤 첫 블록에서 지연 버퍼를 비우고 목표값으로 바로 시작했는가.
    propagation_primed: bool,
    sample_rate: f32,
    /// 채널별 공기 흡음(거리만큼 고역 감쇠). 현장 출력(채널 DSP)에는 걸지 않는다 —
    /// 실제 공기가 흡음하므로 두 번 깎인다.
    air_filters: Vec<crate::audio::dsp::acoustic_physics::AirAbsorptionFilter>,
    /// 채널별 현장 물리 밴드(경계면 저음 증가·룸 모드 피크·근접면 반사 피크·스피커 지향성).
    /// Dart position_eq.dart가 자동 EQ와 **같은 모델**로 계산해 보낸다. 현장에서는 실제 물리가
    /// 자동 EQ 보정을 상쇄하고, 헤드폰에서는 이 밴드가 상쇄한다.
    sim_filters: Vec<[crate::audio::dsp::dsp_utils::EqFilterState; MAX_SIM_BANDS]>,
}

/// 채널당 현장 물리 밴드 수(고정 슬롯 — Dart kFieldPhysicsBandCount와 같아야 한다).
pub const MAX_SIM_BANDS: usize = 8;

/// 같은 고도의 SOFA 측정점들. 방위각([0, 360)) 순으로 정렬돼 있다.
struct ElevationRing {
    elevation: f32,
    entries: Vec<(f32, usize)>,
}

/// 방위각(도)을 [0, 360)으로 정규화한다.
fn normalize_azimuth(azimuth_deg: f32) -> f32 {
    ((azimuth_deg % 360.0) + 360.0) % 360.0
}

/// 측정 위치를 고도 링으로 묶는다(초기화 때 1회, 오디오 스레드 밖).
fn build_elevation_rings(positions: &[SourcePosition]) -> Vec<ElevationRing> {
    let mut rings: Vec<ElevationRing> = Vec::new();
    for (i, p) in positions.iter().enumerate() {
        let az = normalize_azimuth(p.azimuth);
        match rings.iter_mut().find(|r| (r.elevation - p.elevation).abs() < 0.01) {
            Some(r) => r.entries.push((az, i)),
            None => rings.push(ElevationRing { elevation: p.elevation, entries: vec![(az, i)] }),
        }
    }
    rings.sort_by(|a, b| a.elevation.total_cmp(&b.elevation));
    for r in rings.iter_mut() {
        r.entries.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    rings
}

/// 헤드폰 미리듣기에서 흉내 내는 스피커 → 청취 지점 최대 거리(m). 더 멀면 이 거리로 본다.
const MAX_PROPAGATION_DISTANCE_M: f32 = 60.0;
/// 전파 지연이 바뀔 때 옛 지연과 새 지연을 잇는 교차 페이드 길이(초). 페이드 도중 들어온
/// 새 목표는 기다렸다가 끝나는 즉시 가장 최근 목표로 다음 페이드를 시작한다(초기반사 탭과
/// 같은 방식 — 드래그 중 매 프레임 재시작하면 테이프 스톱/스타트처럼 들린다).
const PROPAGATION_XFADE_S: f32 = 0.03;
/// 거리 감쇠 게인 스무딩 시간(초).
const PROPAGATION_GAIN_SMOOTH_S: f32 = 0.01;
/// 거리 감쇠 게인 범위(dB). 청취 지점에 붙은 스피커 같은 극단값에서 폭주하지 않게 막는다.
const PROPAGATION_GAIN_MIN_DB: f32 = -40.0;
const PROPAGATION_GAIN_MAX_DB: f32 = 24.0;

/// 전파 지연이 이만큼(초) 그대로여야 새 지연으로 옮겨 간다. 스피커를 끄는 동안에는 위치가
/// 16ms마다 바뀌므로 지연을 붙잡아 두고, 놓은 뒤 한 번만 교차 페이드한다. 예전에는 매번
/// 교차 페이드해서 두 지연이 섞이는 순간 고역이 서로 지워져 소리가 뚝뚝 끊겼다(실기 보고).
/// 자동 튜닝이 드래그 중 시간 정렬 딜레이를 붙잡아 두는 것과 같은 이유다.
const PROPAGATION_DELAY_SETTLE_S: f32 = 0.15;

/// 채널 하나의 전파 흉내 상태.
#[derive(Clone, Copy)]
struct Propagation {
    /// 마지막으로 받은 지연(샘플). [PROPAGATION_DELAY_SETTLE_S] 동안 그대로면 target이 된다.
    pending_delay: usize,
    /// pending_delay가 그대로인 시간(샘플).
    settle: usize,
    target_delay: usize,
    current_delay: usize,
    prev_delay: usize,
    /// 교차 페이드 진행도(1.0 = 끝남).
    xfade: f32,
    target_gain: f32,
    current_gain: f32,
    /// 스피커 → 청취 지점 거리(m). 공기 흡음 필터가 쓴다(필터가 자체적으로 부드럽게 옮긴다).
    distance_m: f32,
}

impl Default for Propagation {
    fn default() -> Self {
        Self {
            pending_delay: 0,
            settle: 0,
            target_delay: 0,
            current_delay: 0,
            prev_delay: 0,
            xfade: 1.0,
            target_gain: 1.0,
            current_gain: 1.0,
            distance_m: 0.0,
        }
    }
}

impl VirtualMixRoomBinaural {
    /// 48kHz 엔진 기준으로 만든다. 엔진 샘플레이트를 아는 곳에서는 [Self::new_with_rate]를 쓴다.
    pub fn new(num_channels: usize, block_size: usize) -> Self {
        Self::new_with_rate(num_channels, block_size, 48_000.0)
    }

    /// `sample_rate`는 엔진(오디오 장치) 샘플레이트다. HRTF를 이 레이트로 맞춰 쓴다.
    pub fn new_with_rate(num_channels: usize, block_size: usize, sample_rate: f32) -> Self {
        // SOFA HRTF 파일은 빌드 머신의 절대경로(CARGO_MANIFEST_DIR)에 의존하면 배포된
        // 실행 파일에서 경로를 찾지 못해 조용히 폴백된다. 이를 막기 위해 컴파일 타임에
        // 파일 바이트를 바이너리 내부에 직접 임베드(include_bytes!)하여 배포 머신의
        // 파일시스템 경로 존재 여부와 무관하게 항상 로드되도록 한다.
        // sofa-reader 크레이트는 바이트 슬라이스 기반 strict 로더를 제공하지 않으므로,
        // 임베드된 바이트를 엔진 초기화 시점(new(), 오디오 스레드 아님)에 임시 파일로
        // 1회 기록한 뒤 strict_load()로 읽는다.
        const SOFA_BYTES: &[u8] = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/hrtf/mit_kemar_normal_pinna.sofa"
        ));
        let hrtf_db = Self::load_equalized_sofa(SOFA_BYTES, sample_rate);

        let nominal_distance = hrtf_db
            .as_ref()
            .and_then(|db| db.positions.first())
            .map(|p| p.distance)
            .unwrap_or(1.0);
        let ir_len = hrtf_db.as_ref().map(|db| db.ir_length).unwrap_or(2);
        let hrtf_sample_rate = hrtf_db.as_ref().map(|db| db.sample_rate);

        // 정면(0° azimuth, 0° elevation) 실측 HRIR로 초기 채널을 구성한다. 로드 실패 시 무필터 폴백.
        let (init_ir_left, init_ir_right): (Vec<f32>, Vec<f32>) = match &hrtf_db {
            Some(db) => {
                let target = SourcePosition::new(0.0, 0.0, nominal_distance);
                let (idx, _dist) = db.find_nearest(&target);
                match db.get_hrtf_slices(idx) {
                    Some((_pos, left, right)) => (left.to_vec(), right.to_vec()),
                    None => (vec![1.0, 0.0], vec![0.0, 1.0]),
                }
            }
            None => (vec![1.0, 0.0], vec![0.0, 1.0]),
        };

        // 전파 지연 버퍼 길이: 최대 거리를 소리가 가는 시간 + 여유 2샘플.
        let propagation_len = (MAX_PROPAGATION_DISTANCE_M / crate::audio::acoustic::SPEED_OF_SOUND_M_S
            * sample_rate)
            .ceil() as usize
            + 2;

        let mut channels = Vec::new();
        let mut temp_channel_buffers = Vec::new();
        for _ in 0..num_channels {
            channels.push(BinauralChannel::new(&init_ir_left, &init_ir_right, block_size));
            temp_channel_buffers.push(vec![0.0; 8192]);
        }

        // 방위각 공간 인덱스를 초기화 시점에 1회 구축(오디오 콜백에서는 조회만 함).
        let elevation_rings = hrtf_db
            .as_ref()
            .map(|db| build_elevation_rings(&db.positions))
            .unwrap_or_default();
        let elevation_range = match (elevation_rings.first(), elevation_rings.last()) {
            (Some(lo), Some(hi)) => (lo.elevation, hi.elevation),
            _ => (0.0, 0.0),
        };

        Self {
            channels,
            enabled: false,
            temp_channel_buffers,
            mix_left: vec![0.0; 8192],
            mix_right: vec![0.0; 8192],
            current_yaw: 0.0,
            current_pitch: 0.0,
            current_roll: 0.0,
            hrtf_db,
            nominal_distance,
            interp_ir_left: vec![0.0; ir_len],
            interp_ir_right: vec![0.0; ir_len],
            elevation_rings,
            elevation_range,
            channel_enabled: vec![true; num_channels],
            channel_base_azimuth: vec![0.0; num_channels],
            channel_base_elevation: vec![0.0; num_channels],
            applied_elevation: vec![f32::NAN; num_channels],
            // f32::NAN으로 시작해 첫 콜백에서는 모든 채널이 갱신되게 한다.
            applied_azimuth: vec![f32::NAN; num_channels],
            hrir_rebuild_count: 0,
            base_azimuth_dirty: true,
            hrtf_sample_rate,
            propagation: vec![Propagation::default(); num_channels],
            propagation_buffers: vec![vec![0.0; propagation_len]; num_channels],
            propagation_write_idx: 0,
            propagation_len,
            propagation_primed: false,
            sample_rate,
            air_filters: vec![
                crate::audio::dsp::acoustic_physics::AirAbsorptionFilter::new();
                num_channels
            ],
            sim_filters: (0..num_channels)
                .map(|_| std::array::from_fn(|_| crate::audio::dsp::dsp_utils::EqFilterState::new()))
                .collect(),
        }
    }

    /// 채널의 현장 물리 밴드(최대 [MAX_SIM_BANDS]개, 슬롯 고정)를 바꾼다. 모자란 슬롯은 끈다.
    ///
    /// 오디오 스레드(UpdateSpatialConfig)에서 불린다 — 목표값만 바꾸고(Law 1) 계수는
    /// EqFilterState가 샘플마다 부드럽게 옮긴다(Law 3).
    pub fn set_channel_sim_bands(&mut self, ch: usize, bands: &[crate::common::config::EqBand]) {
        let sample_rate = self.sample_rate;
        let Some(filters) = self.sim_filters.get_mut(ch) else { return };
        let off = crate::common::config::EqBand::default();
        for (k, f) in filters.iter_mut().enumerate() {
            f.update(bands.get(k).unwrap_or(&off), sample_rate);
        }
    }

    /// 채널(스피커)에서 청취 지점까지의 거리로 전파를 흉내 낸다: 거리만큼 늦게,
    /// `reference_m`(자동 게인의 기준 거리, acoustic::gain_reference_distance)에서 0dB가
    /// 되도록 거리에 반비례해 작게.
    ///
    /// 오디오 스레드(UpdateSpatialConfig)에서 불린다 — 사전 할당한 상태에 쓰기만 한다(Law 1).
    /// 바뀐 값은 process_interleaved에서 교차 페이드·스무딩으로 옮겨 간다(Law 3).
    pub fn set_channel_propagation(&mut self, ch: usize, distance_m: f32, reference_m: f32) {
        let max_delay = self.propagation_len.saturating_sub(1);
        let sample_rate = self.sample_rate;
        let Some(p) = self.propagation.get_mut(ch) else { return };
        let d = distance_m.clamp(0.0, MAX_PROPAGATION_DISTANCE_M);
        p.distance_m = d;
        let delay = (d / crate::audio::acoustic::SPEED_OF_SOUND_M_S * sample_rate).round() as usize;
        let delay = delay.min(max_delay);
        if delay != p.pending_delay {
            p.pending_delay = delay;
            p.settle = 0;
        }
        let gain_db = (20.0 * (reference_m.max(0.01) / d.max(0.01)).log10())
            .clamp(PROPAGATION_GAIN_MIN_DB, PROPAGATION_GAIN_MAX_DB);
        p.target_gain = 10.0f32.powf(gain_db / 20.0);
    }

    /// 테스트용: 채널의 스피커 → 청취 지점 거리(m). 헤드폰 전파(지연·거리 감쇠·공기 흡음)가 쓴다.
    pub fn debug_propagation_distance(&self, ch: usize) -> f32 {
        self.propagation.get(ch).map(|p| p.distance_m).unwrap_or(0.0)
    }

    /// 좌표가 없는 채널: 전파 흉내를 끈다(지연 0, 0dB).
    pub fn clear_channel_propagation(&mut self, ch: usize) {
        if let Some(p) = self.propagation.get_mut(ch) {
            if p.pending_delay != 0 {
                p.pending_delay = 0;
                p.settle = 0;
            }
            p.target_gain = 1.0;
            p.distance_m = 0.0;
        }
    }

    /// 실제로 쓰고 있는 HRTF의 샘플레이트. 엔진 샘플레이트와 같아야 한다.
    pub fn hrtf_sample_rate(&self) -> Option<f32> {
        self.hrtf_sample_rate
    }

    /// 헤드폰으로 렌더링할 채널을 고른다(전부 true면 예전과 같은 동작).
    ///
    /// 설계는 방 단위로 한다. 지금 보고 있는 방의 스피커만 들려야, 다섯 방이
    /// 한꺼번에 들리는 상태가 되지 않는다. 마스크는 `new()`에서 사전 할당한
    /// 버퍼에 쓰기만 하므로 힙 할당이 없다(Law 1).
    pub fn channel_enabled_mut(&mut self) -> &mut [bool] {
        &mut self.channel_enabled
    }

    /// 채널별 기준 방위각을 갱신한다. 오디오 스레드에서
    /// `UpdateSpatialConfig` 처리 중 호출된다(Law 1: 슬라이스 복사만, 힙
    /// 할당 없음 — 길이가 다르면 짧은 쪽까지만 갱신하고 나머지는 이전 값을
    /// 유지한다).
    pub fn set_channel_base_azimuths(&mut self, azimuths: &[f32]) {
        let n = self.channel_base_azimuth.len().min(azimuths.len());
        self.channel_base_azimuth[..n].copy_from_slice(&azimuths[..n]);
        self.base_azimuth_dirty = true;
    }

    /// 사전 할당된 채널별 방위각 버퍼에 직접 쓰기 위한 가변 접근자.
    ///
    /// `recalculate_binaural_channel_azimuths()`(mixer.rs)가 이 슬라이스에
    /// 바로 써서, 임시 `Vec`을 만들었다가 [`set_channel_base_azimuths`]로
    /// 복사하는 추가 할당을 피한다. 커맨드 핸들러(`UpdateSpatialConfig`)는
    /// 실제 크래시 스택으로 오디오 스레드 위에서 실행됨이 확인된 코드
    /// 경로라(engine.rs 주석 참고), 여기서도 Law 1(무할당)을 지킨다.
    /// 호출 후 반드시 [`mark_base_azimuth_dirty`]를 불러야 재보간이 반영된다.
    pub fn channel_base_azimuth_mut(&mut self) -> &mut [f32] {
        &mut self.channel_base_azimuth
    }

    /// 채널별 (방위각, 고도각) 버퍼를 함께 빌린다. mixer의
    /// `recalculate_binaural_channel_azimuths()`가 한 번에 쓴다(Law 1, 할당 없음).
    /// 호출 후 [`mark_base_azimuth_dirty`]를 불러야 반영된다.
    pub fn channel_base_angles_mut(&mut self) -> (&mut [f32], &mut [f32]) {
        (&mut self.channel_base_azimuth, &mut self.channel_base_elevation)
    }

    /// [`channel_base_azimuth_mut`]로 값을 갱신한 뒤 호출해 다음 오디오
    /// 콜백에서 재보간이 일어나도록 표시한다.
    /// 이 바이노럴 인스턴스가 처리할 수 있는 채널 수.
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// HRIR을 다시 만든 누적 횟수(테스트 검증용).
    pub fn debug_hrir_rebuild_count(&self) -> u64 {
        self.hrir_rebuild_count
    }

    pub fn mark_base_azimuth_dirty(&mut self) {
        self.base_azimuth_dirty = true;
    }

    /// SOFA를 읽고 엔진 샘플레이트에 맞춰 보정한다(audio::hrtf_eq). 결과는 샘플레이트별로
    /// 프로세스 안에 캐시한다 — 보정은 수백 ms 걸리는데 엔진은 재스캔·워치독 복구 때마다
    /// 다시 만들어지기 때문이다. 엔진 초기화 시점(오디오 스레드 밖)에서만 호출된다.
    fn load_equalized_sofa(bytes: &[u8], sample_rate: f32) -> Option<SofaFile> {
        static CACHE: std::sync::OnceLock<std::sync::Mutex<Vec<(u32, SofaFile)>>> =
            std::sync::OnceLock::new();
        let key = sample_rate.round() as u32;
        let cache = CACHE.get_or_init(|| std::sync::Mutex::new(Vec::new()));
        if let Ok(guard) = cache.lock() {
            if let Some((_, db)) = guard.iter().find(|(k, _)| *k == key) {
                return Some(db.clone());
            }
        }
        let mut db = Self::load_embedded_sofa(bytes)?;
        crate::audio::hrtf_eq::equalize(&mut db, sample_rate);
        if let Ok(mut guard) = cache.lock() {
            if !guard.iter().any(|(k, _)| *k == key) {
                guard.push((key, db.clone()));
            }
        }
        Some(db)
    }

    /// 바이너리에 임베드된 SOFA 바이트를 임시 파일에 1회 기록한 뒤 strict_load로 읽는다.
    /// 엔진 초기화 시점(new())에서만 호출되며 오디오 콜백 경로가 아니므로 파일 I/O가 허용된다.
    /// 동일 프로세스 내에서 VirtualMixRoomBinaural 인스턴스가 여러 스레드(테스트 등)에서
    /// 동시에 생성될 수 있으므로, PID만으로는 경로가 충돌한다. 프로세스 전역 원자 카운터를
    /// 더해 인스턴스마다 고유한 임시 경로를 보장한다.
    fn load_embedded_sofa(bytes: &[u8]) -> Option<SofaFile> {
        static INSTANCE_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let instance_id = INSTANCE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let temp_path = std::env::temp_dir().join(format!(
            "atmos_mixer_pro_hrtf_{}_{}.sofa",
            std::process::id(),
            instance_id
        ));
        if let Err(e) = std::fs::write(&temp_path, bytes) {
            eprintln!(
                "[Binaural] 임베드된 SOFA 데이터를 임시 파일에 쓰지 못했습니다({}): {}. 무필터 폴백 IR을 사용합니다.",
                temp_path.display(),
                e
            );
            return None;
        }

        let result = SofaFile::strict_load(&temp_path);
        let _ = std::fs::remove_file(&temp_path);

        match result {
            Ok(db) => Some(db),
            Err(e) => {
                eprintln!(
                    "[Binaural] 임베드된 SOFA HRTF 파싱 실패: {}. 무필터 폴백 IR을 사용합니다.",
                    e
                );
                None
            }
        }
    }

    /// 후보 하나를 최근접 3점 목록에 넣는다(같은 측정점은 한 번만).
    fn consider_nearest(best: &mut [(usize, f32); 3], idx: usize, dist: f32) {
        if best.iter().any(|b| b.0 == idx && b.1.is_finite()) {
            return;
        }
        if dist < best[0].1 {
            best[2] = best[1];
            best[1] = best[0];
            best[0] = (idx, dist);
        } else if dist < best[1].1 {
            best[2] = best[1];
            best[1] = (idx, dist);
        } else if dist < best[2].1 {
            best[2] = (idx, dist);
        }
    }

    /// 후보가 3개보다 적으면 가장 가까운 점으로 채운다(보간 가중치가 그 점으로 모인다).
    fn fill_missing_nearest(best: &mut [(usize, f32); 3]) {
        if !best[1].1.is_finite() {
            best[1] = best[0];
        }
        if !best[2].1.is_finite() {
            best[2] = best[1];
        }
    }

    /// SofaFile::find_three_nearest와 동일한 거리 공식(방위각+고도 구면 거리 + 거리 가중)을
    /// 쓰되, new()에서 만든 고도 링 인덱스로 후보를 좁힌다: 목표 고도를 사이에 두는 링과 그
    /// 바깥 링(최대 4개)에서 방위각 이웃 4점씩. 오디오 콜백에서 호출되며 힙 할당이 없다
    /// (고정 크기 배열만 사용, Law 1).
    fn find_three_nearest_indexed(
        db: &SofaFile,
        rings: &[ElevationRing],
        target: &SourcePosition,
    ) -> [(usize, f32); 3] {
        let positions = &db.positions;
        let mut best = [(0usize, f32::INFINITY); 3];
        if positions.is_empty() || rings.is_empty() {
            return best;
        }
        let az = normalize_azimuth(target.azimuth);
        let upper = rings.partition_point(|r| r.elevation < target.elevation);
        let lo = upper.saturating_sub(2);
        let hi = (upper + 2).min(rings.len());
        for ring in &rings[lo..hi] {
            let n = ring.entries.len();
            if n == 0 {
                continue;
            }
            let i = ring.entries.partition_point(|e| e.0 < az) as isize;
            // 방위각 양옆 이웃 2점씩(링은 원형이라 끝과 처음이 이어진다).
            for k in [-2isize, -1, 0, 1] {
                let j = (i + k).rem_euclid(n as isize) as usize;
                let idx = ring.entries[j].1;
                Self::consider_nearest(&mut best, idx, Self::lookup_distance(&positions[idx], target));
            }
        }
        Self::fill_missing_nearest(&mut best);
        best
    }

    /// 테스트용: 고도 링 인덱스로 찾은 최근접 3개 측정점(인덱스, 거리).
    pub fn debug_nearest_hrirs(&self, azimuth_deg: f32, elevation_deg: f32) -> [(usize, f32); 3] {
        match &self.hrtf_db {
            Some(db) => Self::find_three_nearest_indexed(
                db,
                &self.elevation_rings,
                &SourcePosition::new(azimuth_deg, elevation_deg, self.nominal_distance),
            ),
            None => [(0, f32::INFINITY); 3],
        }
    }

    /// 테스트용: 전체 측정점을 다 훑어 찾은 최근접 3개(인덱스 조회와 비교한다).
    pub fn debug_nearest_hrirs_brute_force(&self, azimuth_deg: f32, elevation_deg: f32) -> [(usize, f32); 3] {
        let mut best = [(0usize, f32::INFINITY); 3];
        if let Some(db) = &self.hrtf_db {
            let target = SourcePosition::new(azimuth_deg, elevation_deg, self.nominal_distance);
            for (idx, pos) in db.positions.iter().enumerate() {
                Self::consider_nearest(&mut best, idx, Self::lookup_distance(pos, &target));
            }
            Self::fill_missing_nearest(&mut best);
        }
        best
    }

    /// sofa-reader crate 내부의 private `lookup_distance`와 동일한 방위각+거리 가중 공식.
    fn lookup_distance(a: &SourcePosition, b: &SourcePosition) -> f32 {
        let angular_deg = a.angular_distance(b);
        if !angular_deg.is_finite() {
            return f32::INFINITY;
        }
        let radial = (a.distance - b.distance).abs();
        if !radial.is_finite() {
            return f32::INFINITY;
        }
        let mean_r = ((a.distance.abs() + b.distance.abs()) * 0.5).max(1e-3);
        let tangential = mean_r * angular_deg.to_radians();
        (tangential * tangential + radial * radial).sqrt()
    }

    pub fn process_interleaved(&mut self, output: &mut [f32], out_channels: usize) {
        if !self.enabled || out_channels < 2 {
            self.propagation_primed = false;
            return;
        }
        
        // Handle 3-DoF Head Tracking via GLOBAL_STATE
        let yaw = f32::from_bits(crate::core::state::GLOBAL_STATE.hrtf_yaw.load(std::sync::atomic::Ordering::Relaxed));
        let pitch = f32::from_bits(crate::core::state::GLOBAL_STATE.hrtf_pitch.load(std::sync::atomic::Ordering::Relaxed));
        let roll = f32::from_bits(crate::core::state::GLOBAL_STATE.hrtf_roll.load(std::sync::atomic::Ordering::Relaxed));
        
        let head_changed = (yaw - self.current_yaw).abs() > 0.01
            || (pitch - self.current_pitch).abs() > 0.01
            || (roll - self.current_roll).abs() > 0.01;

        if head_changed || self.base_azimuth_dirty {
            self.current_yaw = yaw;
            self.current_pitch = pitch;
            self.current_roll = roll;
            self.base_azimuth_dirty = false;

            if let Some(db) = &self.hrtf_db {
                // 실측 SOFA HRIR 룩업: 채널마다 실제 설치 위치 기준 방위각
                // (channel_base_azimuth)에 머리 요각을 결합해 최종 상대
                // 방위각을 만들고, 그 방향에 대해 최근접 3개 실측 위치를
                // 역거리 가중 삼선형 보간한다. 예전에는 이 계산을 콜백당
                // 한 번만 하고 그 결과를 모든 채널에 동일하게 적용했다
                // (헤드 방향만 반영, 채널 위치 무시 — 스피커별 방향감이
                // 재현되지 않는 결함이었다. BINAURAL_SPATIAL_RENDERING_SPEC.md
                // 참고). 이제 채널마다 반복해 각자의 IR을 구한다.
                //
                // Law 1: 힙 할당 없음 — interp_ir_left/right 스크래치 버퍼를
                // 채널마다 덮어써서 재사용한다(채널을 순차 처리하므로 버퍼가
                // N개일 필요가 없다).
                let num_ch = self.channels.len();
                for ch in 0..num_ch {
                    let base_azimuth = self.channel_base_azimuth.get(ch).copied().unwrap_or(0.0);
                    let azimuth_deg = base_azimuth - yaw.to_degrees();
                    // 고도는 스피커 높이에서 온다(머리 요각과는 무관). 데이터셋 범위 밖이면
                    // 가장 가까운 끝 고도로 본다(MIT KEMAR: -40° ~ 90°).
                    let elevation_deg = self
                        .channel_base_elevation
                        .get(ch)
                        .copied()
                        .unwrap_or(0.0)
                        .clamp(self.elevation_range.0, self.elevation_range.1);

                    // 실제로 각도가 바뀐 채널만 다시 만든다. 0.5도는 HRIR
                    // 실측 간격보다 훨씬 작아서 들리는 차이가 없고, 드래그
                    // 중 불필요한 재계산을 거의 다 걷어낸다.
                    // HRIR에는 ITD(좌우 귀 도달 시간차)가 들어 있다. 방위각을
                    // 조금씩 계속 바꾸면 그 시간차도 계속 변하는데, 그건 곧
                    // **움직이는 딜레이**라 피치가 휜다(실기 보고: "스피커를
                    // 움직이면 빨리감기 같은 소리"). 실측 HRTF 격자 간격이
                    // 5도라 그보다 잘게 쪼개도 공간감 이득은 없으므로,
                    // 3도 이상 변했을 때만 새 IR로 간다.
                    let prev = self.applied_azimuth[ch];
                    let prev_elevation = self.applied_elevation[ch];
                    if prev.is_finite()
                        && (azimuth_deg - prev).abs() < 3.0
                        && (elevation_deg - prev_elevation).abs() < 3.0
                    {
                        continue;
                    }
                    self.applied_azimuth[ch] = azimuth_deg;
                    self.applied_elevation[ch] = elevation_deg;
                    self.hrir_rebuild_count += 1;

                    let target = SourcePosition::new(azimuth_deg, elevation_deg, self.nominal_distance);
                    let nbrs = Self::find_three_nearest_indexed(db, &self.elevation_rings, &target);
                    if crate::core::state::debug_flags::binaural() {
                        eprintln!("[디버그] ch={} base_az={} yaw_deg={} target_az={} nbrs={:?}",
                            ch, base_azimuth, yaw.to_degrees(), azimuth_deg, nbrs);
                    }

                    const EPS: f32 = 1e-4;
                    let mut weights = [0.0f32; 3];
                    let mut total = 0.0f32;
                    for (slot, (_idx, dist)) in weights.iter_mut().zip(nbrs.iter()) {
                        let w = 1.0 / (*dist + EPS);
                        *slot = w;
                        total += w;
                    }
                    if total > 0.0 {
                        for w in weights.iter_mut() {
                            *w /= total;
                        }
                    }

                    self.interp_ir_left.fill(0.0);
                    self.interp_ir_right.fill(0.0);
                    for ((idx, _dist), w) in nbrs.iter().zip(weights.iter()) {
                        if let Some((_pos, left, right)) = db.get_hrtf_slices(*idx) {
                            for (dst, src) in self.interp_ir_left.iter_mut().zip(left.iter()) {
                                *dst += src * (*w);
                            }
                            for (dst, src) in self.interp_ir_right.iter_mut().zip(right.iter()) {
                                *dst += src * (*w);
                            }
                        }
                    }

                    self.channels[ch].update_hrtf(&self.interp_ir_left, &self.interp_ir_right);
                }
            } else {
                // SOFA 데이터셋 로드 실패 시 폴백: 기존 합성 시프트 유지(채널
                // 위치는 반영하지 않는다 — 폴백 경로이므로 최소 기능만 유지).
                let shift = yaw.sin() * 0.5; // -0.5 to 0.5
                let dummy_ir_left = [(0.5 - shift).max(0.0), 0.0];
                let dummy_ir_right = [(0.5 + shift).max(0.0), 1.0];

                for ch in &mut self.channels {
                    ch.update_hrtf(&dummy_ir_left, &dummy_ir_right);
                }
            }
        }
        
        let frames = output.len() / out_channels;
        let num_ch = self.channels.len().min(out_channels);
        
        let frames = frames.min(8192);

        // 켜는 순간: 지난번에 켰을 때의 소리가 지연 버퍼에 남아 있으면 안 되고, 목표값에서
        // 바로 시작한다(켜기 전에는 헤드폰으로 나간 소리가 없으니 이어 줄 것도 없다).
        // 고정 크기 버퍼를 비우기만 한다(Law 1).
        if !self.propagation_primed {
            for buf in self.propagation_buffers.iter_mut() {
                buf.fill(0.0);
            }
            for p in self.propagation.iter_mut() {
                p.target_delay = p.pending_delay;
                p.current_delay = p.pending_delay;
                p.prev_delay = p.pending_delay;
                p.xfade = 1.0;
                p.current_gain = p.target_gain;
            }
            self.propagation_primed = true;
        }

        // De-interleave: 채널을 꺼내면서 전파(지연·거리 감쇠)를 입힌다. 헤드폰 입력용
        // 사본에만 걸고 실제 스피커로 나가는 출력(CH3 이후)은 건드리지 않는다.
        let len = self.propagation_len;
        let base = self.propagation_write_idx;
        let xfade_step = 1.0 / (PROPAGATION_XFADE_S * self.sample_rate);
        let gain_coeff = 1.0 / (PROPAGATION_GAIN_SMOOTH_S * self.sample_rate);
        let sample_rate = self.sample_rate;
        let settle_samples = (PROPAGATION_DELAY_SETTLE_S * sample_rate) as usize;
        for ch in 0..num_ch {
            let buf = &mut self.propagation_buffers[ch];
            let p = &mut self.propagation[ch];
            // 새 지연이 한동안 그대로일 때만 옮겨 간다(끄는 동안에는 붙잡아 둔다).
            if p.pending_delay != p.target_delay {
                p.settle = p.settle.saturating_add(frames);
                if p.settle >= settle_samples {
                    p.target_delay = p.pending_delay;
                }
            }
            let dst = &mut self.temp_channel_buffers[ch];
            let air = &mut self.air_filters[ch];
            let sim = &mut self.sim_filters[ch];
            for frame in 0..frames {
                let w = (base + frame) % len;
                buf[w] = output[frame * out_channels + ch];
                if p.xfade >= 1.0 && p.target_delay != p.current_delay {
                    p.prev_delay = p.current_delay;
                    p.current_delay = p.target_delay;
                    p.xfade = 0.0;
                }
                // 읽기 위치는 각 지연에 고정이라 피치가 휘지 않는다(교차 페이드만 한다).
                let now = buf[(w + len - p.current_delay) % len];
                let y = if p.xfade < 1.0 {
                    let old = buf[(w + len - p.prev_delay) % len];
                    let t = p.xfade;
                    p.xfade = (t + xfade_step).min(1.0);
                    // 같은 소리의 가까운 두 지연이라 상관이 높다 → 선형 교차 페이드.
                    old * (1.0 - t) + now * t
                } else {
                    now
                };
                p.current_gain += (p.target_gain - p.current_gain) * gain_coeff;
                // 공기 흡음: 거리만큼 고역을 깎는다(필터가 목표로 부드럽게 옮긴다).
                air.set_distance(p.distance_m, sample_rate);
                let mut s = air.process(y * p.current_gain);
                // 현장 물리 밴드(꺼진 슬롯은 EqFilterState가 바로 통과시킨다).
                for f in sim.iter_mut() {
                    s = f.process(s, sample_rate);
                }
                dst[frame] = s;
            }
        }
        self.propagation_write_idx = (base + frames) % len;
        
        self.mix_left[..frames].fill(0.0);
        self.mix_right[..frames].fill(0.0);
        
        // Process each channel
        for ch in 0..num_ch {
            // 지금 보고 있는 방의 스피커만 헤드폰에 섞는다(channel_enabled_mut 참고).
            if !self.channel_enabled.get(ch).copied().unwrap_or(true) {
                continue;
            }
            self.channels[ch].process_block(&self.temp_channel_buffers[ch][..frames], &mut self.mix_left[..frames], &mut self.mix_right[..frames]);
        }
        
        // Re-interleave
        for frame in 0..frames {
            output[frame * out_channels] = self.mix_left[frame];
            output[frame * out_channels + 1] = self.mix_right[frame];
        }
    }
}
