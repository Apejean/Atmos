use crate::audio::player::SoundInstance;
use crate::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use crate::audio::dsp::dsp_utils::ChannelDspState;

pub enum SpatialGarbage {
    TrackPositions(std::collections::HashMap<String, crate::common::config::Point3D>),
    EqBands(Vec<crate::common::config::EqBand>),
    RoomZones(Vec<crate::common::config::RoomZone>),
    Trajectory(Option<crate::common::config::Trajectory>),
    ChannelPositions(Vec<Option<crate::common::config::Point3D>>),
    ChannelRoomIds(Vec<Option<u32>>),
    BassRouting(Vec<Option<usize>>, Vec<bool>),
    SimBands(Vec<[crate::common::config::EqBand; crate::audio::binaural::MAX_SIM_BANDS]>),
    EarlyReflectionTaps(Vec<[crate::audio::acoustic::EarlyReflectionTap; crate::audio::acoustic::MAX_EARLY_REFLECTION_TAPS]>),
}

/// 베이스 매니지먼트 켜기/끄기·LFE 변경 크로스페이드 길이(초).
const BASS_MANAGEMENT_RAMP_S: f32 = 0.02;

/// 서브 채널 자기 신호(.1 LFE 트랙)의 대역 제한(Hz). 영화·방송 표준 LFE 대역이다.
/// 메인에서 넘어온 저역은 이미 크로스오버 주파수로 잘려 있다.
const LFE_TRACK_LPF_HZ: f32 = 120.0;
/// LFE +10dB 토글의 선형 게인(10^(10/20)).
const LFE_BOOST_LINEAR: f32 = 3.162_277_7;
/// LFE +10dB 켜기/끄기 램프 길이(초).
const LFE_BOOST_RAMP_S: f32 = 0.02;

/// 블록 시작값 `start`에서 `target`으로 샘플당 `step`씩 움직일 때 `frame`번째 샘플의 값.
/// 상태가 없어서 채널 루프 안 어느 채널에서 불러도 같은 궤적이 나온다.
#[inline(always)]
fn bass_management_ramp_at(start: f32, target: f32, step: f32, frame: usize) -> f32 {
    let delta = step * (frame + 1) as f32;
    if target > start {
        (start + delta).min(target)
    } else {
        (start - delta).max(target)
    }
}

pub struct DuckingState {
    pub is_ducking: bool,
    pub ducking_weight: f32, // 1.0 down to 0.3
}

pub struct StartupMuteRamp {
    pub current_gain: f32, // 0.0 -> 1.0 (3초간 서서히 상승)
    pub ramp_step: f32,
}
impl StartupMuteRamp {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            current_gain: 0.0,
            ramp_step: 1.0 / (sample_rate * 3.0), // 3초 분량 샘플 스텝
        }
    }
    /// 블록 안 `frame`번째 프레임에 적용할 램프 게인. 상태를 바꾸지 않는다.
    ///
    /// 예전에는 `apply(sample)` 하나로 "게인 적용 + 전진"을 같이 했는데, 그
    /// 호출이 `for ch { for frame { ... } }` 안에 있어서 **프레임당 한 번이
    /// 아니라 프레임×채널만큼** 전진했다. 그래서 3초로 설계한 부팅 뮤트
    /// 램프가 12채널 장치에서 0.25초 만에 끝났고(= 스피커 충격음 차단이 거의
    /// 무력), 게다가 같은 프레임인데도 채널마다 다른 게인이 걸렸다.
    #[inline(always)]
    pub fn block_gain(&self, frame: usize) -> f32 {
        if self.current_gain >= 1.0 {
            return 1.0;
        }
        (self.current_gain + frame as f32 * self.ramp_step).min(1.0)
    }

    /// 블록을 다 처리한 뒤 **한 번만** 호출해 프레임 수만큼 전진시킨다.
    #[inline(always)]
    pub fn advance_block(&mut self, frames: usize) {
        if self.current_gain < 1.0 {
            self.current_gain = (self.current_gain + frames as f32 * self.ramp_step).min(1.0);
        }
    }
}

pub struct AudioMixer {
    pub instances: Vec<Option<SoundInstance>>, // Fixed capacity object pool
    pub sample_rate: u32,
    pub ducking: DuckingState,
    pub gc_sender: crossbeam_channel::Sender<SoundInstance>,
    pub buf_gc_tx: crossbeam_channel::Sender<Vec<f32>>,
    pub spatial_gc_tx: crossbeam_channel::Sender<SpatialGarbage>,
    pub room_volumes: Vec<Option<(u32, f32)>>,
    pub local_recycle: Vec<Vec<f32>>,
    pub startup_ramp: StartupMuteRamp,
    pub master_mute: bool,
    pub channel_dsp: Vec<ChannelDspState>,
    pub channel_positions: Vec<Option<crate::common::config::Point3D>>,
    /// 채널별 스피커가 속한 방 ID. RoomZone 연결은 acoustic::bind_channel_zone으로 한다.
    pub channel_room_ids: Vec<Option<u32>>,
    /// 채널별 연출용 초기반사 믹스(사용자가 돌린 값 0~1). 실제 적용량은
    /// `refresh_early_ref_mix`가 방 시뮬레이션 몫과 합쳐서 계산한다.
    pub channel_er_mix: Vec<f32>,
    /// 채널별 연출용 믹스 정규화 배율(acoustic::early_reflection_effect_scale).
    pub channel_er_effect_scale: Vec<f32>,
    /// 헤드폰 미리듣기에서 들려줄 방(지금 보고 있는 방). None이면 전체 채널.
    binaural_room: Option<u32>,
    // pan_deg 방위 트림(도 단위). 채널 하드웨어 고정 길이(= channels). DBAP 계산의 가중치 입력(dx/dy)에만
    // 적용되며 물리적 channel_positions/시간정렬은 건드리지 않는다.
    pub channel_pan_deg: Vec<f32>,
    // 채널별 초기반사음 원시 탭(6슬롯). UpdateSpatialConfig로 통째 교체되며 channel_positions와 길이가
    // 다를 수 있음(프론트엔드가 통째로 교체). engine.rs 핸들러가 channel_dsp[ch].taps로 값만 이관한다.
    pub channel_early_ref_taps: Vec<[crate::audio::acoustic::EarlyReflectionTap; crate::audio::acoustic::MAX_EARLY_REFLECTION_TAPS]>,
    pub room_zones: Vec<crate::common::config::RoomZone>,
    /// 리스너(마네킹) 기준점. 3D 룸이 마네킹을 방 중심에 세우므로 그 좌표가
    /// 프론트엔드에서 실려온다. 바이노럴 방위각 계산의 기준점이다.
    pub listener_position: Option<crate::common::config::Point3D>,
    pub trajectory: Option<crate::common::config::Trajectory>,
    pub master_headroom_db: f32,
    pub peak_limiter_enabled: bool,
    pub limiters: Vec<crate::audio::limiter::PeakLimiter>,
    pub temp_room_vols: Vec<f32>,
    pub temp_room_vols_target: Vec<f32>,
    pub temp_room_vols_start: Vec<f32>,
    pub temp_room_vols_phase: Vec<f32>,
    pub channel_spatial_gains: Vec<f32>,
    pub channel_spatial_gains_target: Vec<f32>,
    pub channel_spatial_gains_start: Vec<f32>,
    pub channel_spatial_gains_phase: Vec<f32>,
    pub temp_spatial_weights: Vec<f32>,
    pub analysis_tx: Option<rtrb::Producer<f32>>,
    pub temp_vals: Vec<f32>,
    /// 이번 블록에서 실제로 재생 중인 인스턴스 슬롯 번호.
    ///
    /// 예전에는 프레임 루프 **안에서** 인스턴스 4096칸을 매번 훑었다. 블록마다
    /// 1024 x 4096 = 약 420만 번이라, 재생 중인 트랙이 하나도 없어도 블록당
    /// 6.35ms를 썼다(실시간 예산 21.3ms의 30%). 이제 블록 시작에 한 번만
    /// 목록을 만들고 프레임 루프는 이 목록만 돈다.
    pub temp_active_instances: Vec<usize>,
    
    // Pre-allocated buffers for spatial dsp recalculation
    pub temp_base_delays: Vec<f32>,
    pub temp_base_eqs: Vec<Vec<crate::common::config::EqBand>>,
    pub temp_channel_dists: Vec<f32>,

    pub master_clock: f64,
    pub spatializer: Option<crate::audio::spatial::Spatializer3D>,
    pub reverb: crate::audio::reverb::VirtualRoomReverb,
    pub binaural: crate::audio::binaural::VirtualMixRoomBinaural,
    pub smoothed_trajectory_pos: Option<crate::common::config::Point3D>,
    // Bass Management (LR24) — 방별
    /// 베이스 매니지먼트 적용량(0=바이패스, 1=완전 분할). 서브 지정 변경을
    /// BASS_MANAGEMENT_RAMP_S에 걸쳐 잇는다(Law 3).
    pub bm_mix: f32,
    /// 지금 쓰고 있는 라우팅 표. route[ch] = 그 채널의 저역을 받을 서브(자기 방의 서브).
    /// 서브 자신·서브 없는 방·스피커 없는 채널은 None(audio::bass_route).
    bass_route: Vec<Option<usize>>,
    /// 지금 서브우퍼인 채널들.
    channel_is_sub: Vec<bool>,
    /// 새로 받은 라우팅 표. 적용량이 0으로 내려간 블록 경계에서 위 표와 바꾼다
    /// (옛 서브 페이드아웃 → 교체 → 새 서브 페이드인).
    bass_route_pending: Vec<Option<usize>>,
    channel_is_sub_pending: Vec<bool>,
    bass_route_dirty: bool,
    pub crossovers: Vec<crate::audio::crossover::LinkwitzRiley24>,
    /// LFE +10dB 토글(베이스 매니지먼트 패널). 켜면 서브 채널 **자기 신호**(.1 LFE 트랙)를
    /// 120Hz 로우패스 **이후**에 +10dB 올린다. 메인에서 넘어온 저역에는 걸지 않는다.
    /// 서브 레벨은 원래 최종 출력단·하드웨어에서 맞추는 것이고, 이건 바이노럴 미리듣기나
    /// 소프트웨어로 맞춰야 할 때를 위한 스위치다.
    pub lfe_boost_enabled: bool,
    /// LFE 부스트의 현재 선형 게인(1.0 <-> LFE_BOOST_LINEAR 사이를 램프로 오간다).
    lfe_boost_gain: f32,
    /// 채널별 서브 자기 신호용 120Hz 로우패스(LR4 저역 쪽만 쓴다). 서브가 된 채널만 쓴다.
    lfe_track_lpfs: Vec<crate::audio::crossover::LinkwitzRiley24>,
    // 채널별 서브우퍼로 보낼 저역 합산 버퍼. 서브가 된 채널 것만 쓴다
    // (Law 1: new()에서 사전 할당, binaural.rs의 8192 프레임 관례).
    lfe_sub_mix: Vec<Vec<f32>>,
}

impl AudioMixer {
    /// `block_size`: 이 엔진이 실제로 처리할 콜백당 프레임 수(하드웨어 버퍼
    /// 크기, 예: 1024). 바이노럴 렌더러의 FFT/오버랩-애드 버퍼를 이 크기에
    /// 맞춰 사전 할당하는 데만 쓰인다.
    ///
    /// 이 값이 실제 `process()` 호출 시 넘어오는 프레임 수와 다르면
    /// `BinauralChannel`의 오버랩-애드 컨볼루션이 깨진다(매 콜백 오버랩
    /// 테일을 처음부터 다시 만드는 구조라, FFT 크기가 실제 프레임 수보다
    /// 훨씬 크면 이전 블록이 넘겨준 테일 대부분을 매번 버리게 되어 출력이
    /// 완전히 무음이 되거나 뒤섞인다). 예전에는 이 값이 실제 하드웨어 버퍼
    /// 크기(1024~2048)와 무관하게 8192로 하드코딩되어 있었다 — 바이노럴이
    /// UI 토글이 없어 실제 엔진 루프로 한 번도 제대로 실행된 적이 없어서
    /// 지금까지 드러나지 않은 잠재 결함이었다(수치 검증:
    /// `src/bin/binaural_debug.rs`, 회귀 테스트:
    /// `tests/test_binaural_channel_azimuth.rs`).
    pub fn new(sample_rate: u32, channels: usize, block_size: usize, gc_sender: crossbeam_channel::Sender<SoundInstance>, analysis_tx: Option<rtrb::Producer<f32>>) -> Self {
        let (buf_gc_tx, buf_gc_rx) = crossbeam_channel::bounded::<Vec<f32>>(65536);
        std::thread::spawn(move || {
            while let Ok(_buf) = buf_gc_rx.recv() {
                // Buffer is dropped here in a background thread, preventing heap deallocation in the audio thread
            }
        });

        let (spatial_gc_tx, spatial_gc_rx) = crossbeam_channel::bounded::<SpatialGarbage>(1024);
        std::thread::spawn(move || {
            while let Ok(_garbage) = spatial_gc_rx.recv() {
                // Vecs drop here, preventing OS allocations in the audio thread
            }
        });

        let mut instances = Vec::with_capacity(4096);
        for _ in 0..4096 {
            instances.push(None);
        }
        let mut channel_dsp = Vec::with_capacity(channels);
        for _ in 0..channels {
            channel_dsp.push(ChannelDspState::new());
        }
        let mut channel_positions = vec![None; channels];
        let mut room_zones = Vec::new();
        let mut trajectory = None;
        let mut master_headroom_db = 0.0;
        let mut peak_limiter_enabled = true;
        let mut limiters = Vec::with_capacity(channels);
        for _ in 0..channels {
            limiters.push(crate::audio::limiter::PeakLimiter::new(sample_rate as f32, 1.0, 500.0, 0.99));
        }

        if let Ok(config_guard) = GLOBAL_STATE.config.read() {
            if let Some(config) = config_guard.as_ref() {
                for (&ch_key, setting) in &config.mono_configs {
                    if ch_key > 0 {
                        let ch_idx = (ch_key - 1) as usize;
                        if ch_idx < channel_dsp.len() {
                            channel_dsp[ch_idx].update_delay_target(setting.delay_ms);
                            channel_dsp[ch_idx].update_eq_targets(&setting.eq_bands.clone(), sample_rate as f32);
                            channel_positions[ch_idx] = setting.position.clone();
                            channel_dsp[ch_idx].phase_invert = setting.phase_invert;
                            channel_dsp[ch_idx].set_gain_db(setting.gain_db);
                        }
                    }
                }
                for (&ch_key, setting) in &config.stereo_configs {
                    if ch_key > 0 {
                        let ch_idx1 = (ch_key - 1) as usize;
                        let ch_idx2 = ch_idx1 + 1;
                        if ch_idx1 < channel_dsp.len() {
                            channel_dsp[ch_idx1].update_delay_target(setting.delay_ms);
                            channel_dsp[ch_idx1].update_eq_targets(&setting.eq_bands.clone(), sample_rate as f32);
                            channel_positions[ch_idx1] = setting.position.clone();
                            channel_dsp[ch_idx1].phase_invert = setting.phase_invert;
                            channel_dsp[ch_idx1].set_gain_db(setting.gain_db);
                        }
                        if ch_idx2 < channel_dsp.len() {
                            channel_dsp[ch_idx2].update_delay_target(setting.delay_ms);
                            channel_dsp[ch_idx2].update_eq_targets(&setting.eq_bands.clone(), sample_rate as f32);
                            channel_positions[ch_idx2] = setting.position.clone();
                            channel_dsp[ch_idx2].phase_invert = setting.phase_invert;
                            channel_dsp[ch_idx2].set_gain_db(setting.gain_db);
                        }
                    }
                }
                for (&ch_key, setting) in &config.multi_configs {
                    if ch_key > 0 {
                        let ch_idx_base = (ch_key - 1) as usize;
                        for i in 0..6 {
                            let ch_idx = ch_idx_base + i;
                            if ch_idx < channel_dsp.len() {
                                channel_dsp[ch_idx].update_delay_target(setting.delay_ms);
                                channel_dsp[ch_idx].update_eq_targets(&setting.eq_bands.clone(), sample_rate as f32);
                                channel_positions[ch_idx] = setting.position.clone();
                                channel_dsp[ch_idx].phase_invert = setting.phase_invert;
                                channel_dsp[ch_idx].set_gain_db(setting.gain_db);
                            }
                        }
                    }
                }
                
                // Auto Boundary EQ and Acoustic Delay is now handled by recalculate_spatial_dsp()
                // --------------------------------------------------------------------
                room_zones = config.room_zones.clone();
                trajectory = config.global_trajectory.clone();
                master_headroom_db = config.master_headroom_db;
                peak_limiter_enabled = config.peak_limiter_enabled;
            }
        }

        let mut mixer = Self {
            instances,
            sample_rate,
            ducking: DuckingState {
                is_ducking: false,
                ducking_weight: 1.0,
            },
            gc_sender,
            buf_gc_tx,
            spatial_gc_tx,
            room_volumes: vec![None; channels],
            local_recycle: Vec::with_capacity(65536),
            startup_ramp: StartupMuteRamp::new(sample_rate as f32),
            master_mute: false,
            channel_dsp,
            channel_positions,
            channel_room_ids: Vec::new(),
            channel_er_mix: vec![0.0; channels],
            channel_er_effect_scale: vec![0.0; channels],
            binaural_room: None,
            channel_pan_deg: vec![0.0; channels],
            channel_early_ref_taps: vec![[crate::audio::acoustic::EarlyReflectionTap::default(); crate::audio::acoustic::MAX_EARLY_REFLECTION_TAPS]; channels],
            listener_position: None,
            room_zones,
            trajectory: trajectory.clone(),
            master_headroom_db,
            peak_limiter_enabled,
            limiters,
            temp_room_vols: vec![1.0; 4096],
            temp_room_vols_target: vec![1.0; 4096],
            temp_room_vols_start: vec![1.0; 4096],
            temp_room_vols_phase: vec![1.0; 4096],
            channel_spatial_gains: vec![1.0; channels],
            channel_spatial_gains_target: vec![1.0; channels],
            channel_spatial_gains_start: vec![1.0; channels],
            channel_spatial_gains_phase: vec![1.0; channels],
            temp_spatial_weights: vec![0.0; channels],
            analysis_tx,
            temp_vals: vec![0.0; channels],
            temp_active_instances: Vec::with_capacity(4096),
            temp_base_delays: vec![0.0; channels],
            temp_base_eqs: (0..channels).map(|_| Vec::with_capacity(32)).collect(),
            temp_channel_dists: vec![0.0; channels],
            master_clock: 0.0,
            spatializer: None,
            reverb: crate::audio::reverb::VirtualRoomReverb::new(sample_rate as f32),
            // HRTF를 엔진 샘플레이트로 맞춰 쓴다(audio::hrtf_eq).
            binaural: crate::audio::binaural::VirtualMixRoomBinaural::new_with_rate(
                channels,
                block_size,
                sample_rate as f32,
            ),
            smoothed_trajectory_pos: trajectory.as_ref().map(|t| t.current_position.clone()),
            bm_mix: 0.0,
            // 서브가 지정되기 전(엔진 기동 직후)에는 베이스 매니지먼트가 없다. 서브 지정은
            // 공간 설정 payload로 들어온다(simple.rs api_update_spatial_config_json).
            bass_route: Vec::new(),
            channel_is_sub: Vec::new(),
            bass_route_pending: Vec::new(),
            channel_is_sub_pending: Vec::new(),
            bass_route_dirty: false,
            lfe_boost_enabled: false,
            lfe_boost_gain: 1.0,
            lfe_track_lpfs: vec![crate::audio::crossover::LinkwitzRiley24::new(); channels],
            crossovers: vec![crate::audio::crossover::LinkwitzRiley24::new(); channels],
            lfe_sub_mix: vec![vec![0.0; 8192]; channels],
        };

        for c in mixer.crossovers.iter_mut() {
            c.set_crossover_freq(80.0, sample_rate as f32);
        }
        for f in mixer.lfe_track_lpfs.iter_mut() {
            f.set_crossover_freq(LFE_TRACK_LPF_HZ, sample_rate as f32);
        }
        
        mixer.recalculate_spatial_dsp();

        // 사용자가 켜 둔 바이노럴 상태를 새 믹서가 이어받는다(state.rs 참고).
        mixer.binaural.enabled = crate::core::state::GLOBAL_STATE
            .binaural_enabled
            .load(std::sync::atomic::Ordering::Relaxed);
        mixer.master_mute = crate::core::state::GLOBAL_STATE
            .master_mute
            .load(std::sync::atomic::Ordering::Relaxed);

        // 새 엔진의 믹서다. 옛 엔진이 남긴 위치를 지우고, 표를 여기(오디오 스레드 밖)서 초기화해 둔다.
        crate::audio::playback_cursor::CURSOR_TABLE.clear_all();

        mixer
    }

    pub fn process(&mut self, output: &mut [f32], out_channels: usize) {
        if out_channels == 0 || output.is_empty() {
            return;
        }

        // Clear output buffer
        for sample in output.iter_mut() {
            *sample = 0.0;
        }

        let frames = output.len() / out_channels;

        let fade_frames = (self.sample_rate as f32 * 0.3) as usize; // 300ms fade
        let duck_down_frames = (self.sample_rate as f32 * 0.15) as usize; // 150ms duck down
        let duck_up_frames = (self.sample_rate as f32 * 0.3) as usize; // 300ms duck up
        
        self.master_clock += frames as f64;


        // Check if any SFX is playing (not loop)
        let has_sfx = self.instances.iter().any(|inst| {
            if let Some(inst) = inst {
                !inst.is_loop && inst.is_playing && !inst.is_stopping
            } else {
                false
            }
        });

        let is_exhib = GLOBAL_STATE.is_exhibition_mode.load(Ordering::Relaxed);
        if is_exhib {
            self.ducking.is_ducking = false;
        } else {
            if has_sfx && !self.ducking.is_ducking {
                self.ducking.is_ducking = true;
            } else if !has_sfx && self.ducking.is_ducking && self.ducking.ducking_weight <= 0.3 {
                self.ducking.is_ducking = false; // Start unducking
            }
        }

        self.temp_room_vols_target.fill(1.0);
        for (i, inst_opt) in self.instances.iter_mut().enumerate() {
            if let Some(inst) = inst_opt {
                if inst.is_playing {
                    for (rid, rvol) in self.room_volumes.iter().flatten() {
                        if *rid == inst.room_id {
                            self.temp_room_vols_target[i] = *rvol;
                            break;
                        }
                    }
                    
                    if inst.output_channel == usize::MAX && inst.current_position.is_some() {
                        // inst.spatial_gains는 SoundInstance 생성 시 고정 크기로 사전 할당되므로
                        // (Dante/MADI 등 128채널 초과 인터페이스에서도) OOB panic 방지를 위해 하한 클램프.
                        let active_ch = out_channels.min(self.channel_positions.len()).min(inst.spatial_gains.len());
                        // if inst.spatial_gains.len() != active_ch {
                            // inst.spatial_gains.resize(active_ch, 0.0);
                            // inst.spatial_gains_target.resize(active_ch, 0.0);
                        // }
                        
                        let default_pos = crate::common::config::Point3D::default();
                        let pos = inst.current_position.as_ref().unwrap_or(&default_pos);
                        let mut sum_sq = 0.0;
                        let mut min_dist = f32::MAX;
                        let r_blur = 2.0f32;
                        
                        for ch in 0..active_ch {
                            if let Some(c_pos) = &self.channel_positions[ch] {
                                let dx = c_pos.x - pos.x;
                                let dy = c_pos.y - pos.y;
                                let dz = c_pos.z - pos.z;
                                let dist = (dx*dx + dy*dy + dz*dz).sqrt();
                                if dist < min_dist { min_dist = dist; }
                                let effective_blur = r_blur * (1.0 + pos.size * 3.0);
                                let weight = 1.0 / (dist.powi(2) + effective_blur.powi(2));
                                inst.spatial_gains_target[ch] = weight;
                                sum_sq += weight * weight;
                            } else {
                                inst.spatial_gains_target[ch] = 0.0;
                            }
                        }
                        
                        let norm_factor = if sum_sq > 0.0 { 1.0 / sum_sq.sqrt() } else { 0.0 };
                        let distance_attenuation = 1.0 / min_dist.max(1.0);
                        for ch in 0..active_ch {
                            let pan_ratio = inst.spatial_gains_target[ch] * norm_factor;
                            inst.spatial_gains_target[ch] = pan_ratio * distance_attenuation;
                        }
                    }
                }
            }
        }

        let active_ch = out_channels.min(self.channel_spatial_gains_target.len());
        
        self.channel_spatial_gains_target[..active_ch].fill(1.0);
        self.temp_spatial_weights[..active_ch].fill(0.0);

        if let Some(traj) = &self.trajectory {
            if let Some(current_pos) = &mut self.smoothed_trajectory_pos {
                // 1-pole Low-pass filter (Exponential Smoothing) for 60fps OSC
                let alpha = 0.15; // Smooth over chunks to remove zipper noise
                current_pos.x += alpha * (traj.current_position.x - current_pos.x);
                current_pos.y += alpha * (traj.current_position.y - current_pos.y);
                current_pos.z += alpha * (traj.current_position.z - current_pos.z);
            } else {
                self.smoothed_trajectory_pos = Some(traj.current_position.clone());
            }
        } else {
            self.smoothed_trajectory_pos = None;
        }

        let mut sum_sq = 0.0;
        let mut min_dist = f32::MAX;
        let blur_radius = 2.0f32;

        for ch in 0..active_ch {
            let mut gain = 1.0;
            if ch < self.channel_positions.len() {
                if let Some(pos) = &self.channel_positions[ch] {
                    // 1. 스피커가 속한 방에 연결(acoustic::bind_channel_zone 참고)
                    let bound_room_id = crate::audio::acoustic::bind_channel_zone(
                        &self.room_zones,
                        self.channel_room_ids.get(ch).copied().flatten(),
                        pos,
                    )
                    .map(|z| z.room_id);
                    if let Some(rid) = bound_room_id {
                        for (room_id, rvol) in self.room_volumes.iter().flatten() {
                            if *room_id == rid {
                                gain *= *rvol;
                                break;
                            }
                        }
                    }
                    
                    // 2. Trajectory DBAP Weight Collection
                    if let Some(traj) = &self.trajectory {
                        let mut in_target_room = true;
                        if let Some(target_zone_str) = &traj.target_room_zone_id {
                            let target_rid = target_zone_str.parse::<u32>().unwrap_or_else(|_| crate::common::utils::hash_id(target_zone_str));
                            if bound_room_id != Some(target_rid) {
                                in_target_room = false;
                            }
                        }

                        if in_target_room {
                            let smoothed_pos = self.smoothed_trajectory_pos.as_ref().unwrap_or(&traj.current_position);

                            // pan_deg 방위 트림: 바인딩된 RoomZone 수평 중심을 피벗으로 DBAP 가중치 계산용
                            // 유효 위치(eff_x/eff_y)만 회전시킨다. 물리적 pos.x/pos.y 자체는 불변(시간정렬/
                            // off-axis EQ는 recalculate_spatial_dsp()에서 원본 좌표를 그대로 사용).
                            let pan_deg = self.channel_pan_deg.get(ch).copied().unwrap_or(0.0);
                            let (eff_x, eff_y) = if pan_deg != 0.0 {
                                match bound_room_id.and_then(|rid| self.room_zones.iter().find(|z| z.room_id == rid)) {
                                    Some(zone) => {
                                        let pivot_x = (zone.boundary_min.x + zone.boundary_max.x) * 0.5;
                                        let pivot_y = (zone.boundary_min.y + zone.boundary_max.y) * 0.5;
                                        let theta = pan_deg.to_radians();
                                        let (sin_t, cos_t) = theta.sin_cos();
                                        let dx0 = pos.x - pivot_x;
                                        let dy0 = pos.y - pivot_y;
                                        (pivot_x + dx0 * cos_t - dy0 * sin_t, pivot_y + dx0 * sin_t + dy0 * cos_t)
                                    }
                                    None => (pos.x, pos.y), // 미바인딩 시 무동작
                                }
                            } else {
                                (pos.x, pos.y)
                            };

                            let dx = eff_x - smoothed_pos.x;
                            let dy = eff_y - smoothed_pos.y;
                            let dz = pos.z - smoothed_pos.z;
                            let dist = (dx*dx + dy*dy + dz*dz).sqrt();
                            
                            if dist < min_dist {
                                min_dist = dist;
                            }
                            
                            let effective_blur = blur_radius * (1.0 + traj.current_position.size * 3.0);
                            let weight = 1.0 / (dist.powi(2) + effective_blur.powi(2));
                            self.temp_spatial_weights[ch] = weight;
                            sum_sq += weight * weight;
                        } else {
                            self.temp_spatial_weights[ch] = 0.0;
                        }
                    }
                }
            }
            self.channel_spatial_gains_target[ch] = gain;
        }

        // Apply DBAP Normalization & Global Distance Attenuation
        if self.trajectory.is_some() {
            let norm_factor = if sum_sq > 0.0 { 1.0 / sum_sq.sqrt() } else { 0.0 };
            let distance_attenuation = 1.0 / min_dist.max(1.0);

            for ch in 0..active_ch {
                let pan_ratio = self.temp_spatial_weights[ch] * norm_factor;
                self.channel_spatial_gains_target[ch] *= pan_ratio * distance_attenuation;
            }
        }

        let mut temp_vals = std::mem::take(&mut self.temp_vals);

        // 재생 중인 슬롯만 한 번 추려 둔다(위 temp_active_instances 주석 참고).
        let mut active_instances = std::mem::take(&mut self.temp_active_instances);
        active_instances.clear();
        for (i, inst_opt) in self.instances.iter().enumerate() {
            if let Some(inst) = inst_opt {
                if inst.is_playing {
                    active_instances.push(i);
                }
            }
        }
        for frame in 0..frames {
            // Anti-zipper smoothing for spatial automation (~4ms time constant) and 50ms Equal-Power Crossfade for Scene changes
            let fade_step = 1.0 / (self.sample_rate as f32 * 0.05); // 50ms
            
            for ch in 0..active_ch {
                if (self.channel_spatial_gains_target[ch] - self.channel_spatial_gains[ch]).abs() > 0.1 && self.channel_spatial_gains_phase[ch] >= 1.0 {
                    self.channel_spatial_gains_start[ch] = self.channel_spatial_gains[ch];
                    self.channel_spatial_gains_phase[ch] = 0.0;
                }
                
                if self.channel_spatial_gains_phase[ch] < 1.0 {
                    self.channel_spatial_gains_phase[ch] = (self.channel_spatial_gains_phase[ch] + fade_step).min(1.0);
                    let p = self.channel_spatial_gains_phase[ch];
                    let s = self.channel_spatial_gains_start[ch];
                    let t = self.channel_spatial_gains_target[ch];
                    self.channel_spatial_gains[ch] = s * (p * std::f32::consts::FRAC_PI_2).cos() + t * (p * std::f32::consts::FRAC_PI_2).sin();
                } else {
                    self.channel_spatial_gains[ch] += 0.005 * (self.channel_spatial_gains_target[ch] - self.channel_spatial_gains[ch]);
                }
                
                if ch < GLOBAL_STATE.spatial_gains.len() {
                    GLOBAL_STATE.spatial_gains[ch].store(self.channel_spatial_gains[ch].to_bits(), Ordering::Relaxed);
                }
            }

            // Update ducking weight per frame
            if self.ducking.is_ducking {
                if self.ducking.ducking_weight > 0.3 {
                    self.ducking.ducking_weight -= 0.7 / duck_down_frames as f32;
                }
                if self.ducking.ducking_weight < 0.3 {
                    self.ducking.ducking_weight = 0.3;
                }
            } else {
                if self.ducking.ducking_weight < 1.0 {
                    self.ducking.ducking_weight += 0.7 / duck_up_frames as f32;
                }
                if self.ducking.ducking_weight > 1.0 {
                    self.ducking.ducking_weight = 1.0;
                }
            }

            for idx in 0..active_instances.len() {
                let i = active_instances[idx];
                let instance = match &mut self.instances[i] {
                    Some(inst) => inst,
                    None => continue,
                };

                // 블록 도중 재생이 끝날 수 있으므로 여기서도 확인한다.
                if !instance.is_playing {
                    continue;
                }

                if (self.temp_room_vols_target[i] - self.temp_room_vols[i]).abs() > 0.1 && self.temp_room_vols_phase[i] >= 1.0 {
                    self.temp_room_vols_start[i] = self.temp_room_vols[i];
                    self.temp_room_vols_phase[i] = 0.0;
                }
                
                if self.temp_room_vols_phase[i] < 1.0 {
                    self.temp_room_vols_phase[i] = (self.temp_room_vols_phase[i] + fade_step).min(1.0);
                    let p = self.temp_room_vols_phase[i];
                    let s = self.temp_room_vols_start[i];
                    let t = self.temp_room_vols_target[i];
                    self.temp_room_vols[i] = s * (p * std::f32::consts::FRAC_PI_2).cos() + t * (p * std::f32::consts::FRAC_PI_2).sin();
                } else {
                    self.temp_room_vols[i] += 0.005 * (self.temp_room_vols_target[i] - self.temp_room_vols[i]);
                }
                
                if instance.output_channel == usize::MAX {
                    // OOB panic 방지: spatial_gains 고정 배열 크기로 하한 클램프 (b4a9293 패턴).
                    let active_ch = out_channels.min(self.channel_positions.len()).min(instance.spatial_gains.len());
                    for ch in 0..active_ch {
                        instance.spatial_gains[ch] += 0.005 * (instance.spatial_gains_target[ch] - instance.spatial_gains[ch]);
                    }
                }

                // Update fade weight
                if instance.is_stopping {
                    instance.fade_weight -= 1.0 / fade_frames as f32;
                    if instance.fade_weight <= 0.0 {
                        instance.fade_weight = 0.0;
                        instance.is_playing = false;
                        continue;
                    }
                } else if instance.stream_receiver.is_none() || !instance.stream_buffer.is_empty() {
                    // 스트리밍은 첫 묶음이 오기 전에는 올리지 않는다. 재개는 시작 위치까지 디코딩해 버린
                    // 뒤에야 첫 묶음이 와서, 그동안 올라가 버리면 파형 중간에서 큰 크기로 시작한다(딸깍).
                    instance.fade_weight += 1.0 / fade_frames as f32;
                    if instance.fade_weight > 1.0 {
                        instance.fade_weight = 1.0;
                    }
                }

                let smoothed_volume = instance.volume_smoother.get_next();
                let mut current_vol = smoothed_volume * instance.fade_weight * self.temp_room_vols[i];
                if instance.is_loop {
                    current_vol *= self.ducking.ducking_weight; // Ducking only affects BGM
                }

                let step = instance.stream_sample_rate as f64 / self.sample_rate as f64;

                let channels = (instance.stream_channels as usize).max(1);

                let mut idx_f = instance.cursor;
                let mut idx_base = idx_f as usize;
                let mut frac = (idx_f - (idx_base as f64)) as f32;
                let mut idx_i = idx_base * channels;

                let ch_limit = channels.min(temp_vals.len());
                let vals = &mut temp_vals[..ch_limit];
                let mut has_sample = false;

                if let Some(stream_rx) = &mut instance.stream_receiver {
                    if idx_i >= instance.stream_buffer.len() {
                        match stream_rx.pop() {
                            Ok(new_chunk) => {
                                instance.anti_click_multiplier = 1.0;
                                let frames_in_chunk = (instance.stream_buffer.len() / channels) as f64;
                                instance.position_base_frames += frames_in_chunk;
                                let old_chunk = std::mem::replace(&mut instance.stream_buffer, new_chunk);
                                if let Err(e) = self.buf_gc_tx.try_send(old_chunk) {
                                    let v = e.into_inner();
                                    if self.local_recycle.len() < self.local_recycle.capacity() {
                                        self.local_recycle.extend(std::iter::once(v));
                                    } else {
                                        // Emergency buffer leak
                                        std::mem::forget(v);
                                    }
                                }
                                instance.cursor -= frames_in_chunk;
                                if instance.cursor < 0.0 {
                                    instance.cursor = 0.0;
                                }
                                idx_f = instance.cursor;
                                idx_base = idx_f as usize;
                                frac = (idx_f - (idx_base as f64)) as f32;
                                idx_i = idx_base * channels;
                            }
                            Err(rtrb::PopError::Empty) => {
                                // 디코더 스레드가 끝났고(파일 끝·오류) 남은 묶음도 없으면 이 스트림은 끝났다.
                                // 묶음이 늦을 뿐이면 보내는 쪽이 살아 있다. 보내는 쪽이 끝난 뒤에는 더 들어오지
                                // 않으므로 비어 있는지는 그다음에 본다. 둘 다 원자 읽기다(할당·잠금 없음).
                                if !instance.is_stopping && stream_rx.is_abandoned() && stream_rx.is_empty() {
                                    instance.is_stopping = true;
                                }
                            }
                        }
                    }

                    if idx_i < instance.stream_buffer.len() {
                        has_sample = true;
                        let next_idx = if idx_i + channels < instance.stream_buffer.len() {
                            idx_i + channels
                        } else {
                            idx_i
                        };
                        for (ch, val) in vals.iter_mut().enumerate().take(ch_limit) {
                            let idx0 = if idx_i >= channels { idx_i - channels } else { idx_i };
                            let idx3 = if next_idx + channels < instance.stream_buffer.len() { next_idx + channels } else { next_idx };

                            let s0 = instance.stream_buffer.get(idx0 + ch).copied().unwrap_or(0.0);
                            let s1 = instance.stream_buffer.get(idx_i + ch).copied().unwrap_or(0.0);
                            let s2 = instance.stream_buffer.get(next_idx + ch).copied().unwrap_or(s1);
                            let s3 = instance.stream_buffer.get(idx3 + ch).copied().unwrap_or(s2);
                            
                            *val = crate::audio::dsp::dsp_utils::interpolate_hermite(s0, s1, s2, s3, frac);
                            if let Some(last) = instance.last_samples.get_mut(ch) {
                                *last = *val;
                            }
                        }
                    } else {
                        has_sample = true;
                        instance.anti_click_multiplier *= 0.95; // 1-pole non-linear fade
                        for (ch, val) in vals.iter_mut().enumerate().take(ch_limit) {
                            *val = instance.last_samples.get(ch).copied().unwrap_or(0.0) * instance.anti_click_multiplier;
                        }
                    }
                } else if let Some(data) = &instance.data {
                    if idx_i >= data.samples.len() && instance.is_loop {
                        let frames_in_data = (data.samples.len() / channels) as f64;
                        instance.cursor -= frames_in_data;
                        if instance.cursor < 0.0 {
                            instance.cursor = 0.0;
                        }
                        idx_f = instance.cursor;
                        idx_base = idx_f as usize;
                        frac = (idx_f - (idx_base as f64)) as f32;
                        idx_i = idx_base * channels;
                    }

                    if idx_i < data.samples.len() {
                        has_sample = true;
                        let mut next_idx = idx_i + channels;
                        if next_idx >= data.samples.len() {
                            if instance.is_loop {
                                next_idx = 0;
                            } else {
                                next_idx = idx_i;
                            }
                        }
                        for (ch, val) in vals.iter_mut().enumerate().take(ch_limit) {
                            let idx0 = if idx_i >= channels { idx_i - channels } else { if instance.is_loop && data.samples.len() >= channels { data.samples.len() - channels } else { idx_i } };
                            let idx3 = if next_idx + channels < data.samples.len() { next_idx + channels } else { if instance.is_loop { 0 } else { next_idx } };

                            let s0 = data.samples.get(idx0 + ch).copied().unwrap_or(0.0);
                            let s1 = data.samples.get(idx_i + ch).copied().unwrap_or(0.0);
                            let s2 = data.samples.get(next_idx + ch).copied().unwrap_or(s1);
                            let s3 = data.samples.get(idx3 + ch).copied().unwrap_or(s2);
                            *val = crate::audio::dsp::dsp_utils::interpolate_hermite(s0, s1, s2, s3, frac);
                        }
                    }
                }

                if has_sample {
                    if instance.output_channel == usize::MAX && instance.current_position.is_some() {
                        // Object Mode
                        let mut sum = 0.0;
                        for val in vals.iter().take(ch_limit) {
                            sum += *val;
                        }
                        let mono_val = sum / ch_limit as f32;

                        // OOB panic 방지: spatial_gains 고정 배열 크기로 하한 클램프 (b4a9293 패턴).
                        let active_ch = out_channels.min(self.channel_positions.len()).min(instance.spatial_gains.len());
                        for hw_ch in 0..active_ch {
                            let is_enabled = if hw_ch < GLOBAL_STATE.enabled_channels.len() {
                                GLOBAL_STATE.enabled_channels[hw_ch].load(Ordering::Relaxed)
                            } else {
                                false
                            };
                            let out_idx = frame * out_channels + hw_ch;
                            if is_enabled && out_idx < output.len() {
                                output[out_idx] += mono_val * current_vol * instance.spatial_gains[hw_ch];
                            }
                        }
                    } else if !instance.output_stereo && ch_limit > 1 {
                        // Downmix all to Mono for backwards compatibility if output_stereo is false
                        let mut sum = 0.0;
                        for val in vals.iter().take(ch_limit) {
                            sum += *val;
                        }
                        let mono_val = sum / ch_limit as f32;

                        let hw_ch = instance.output_channel;
                        if hw_ch < out_channels {
                            let is_enabled = if hw_ch < GLOBAL_STATE.enabled_channels.len() {
                                GLOBAL_STATE.enabled_channels[hw_ch].load(Ordering::Relaxed)
                            } else {
                                false
                            };
                            let out_idx = frame * out_channels + hw_ch;
                            if is_enabled && out_idx < output.len() {
                                output[out_idx] += mono_val * current_vol;
                            }
                        }
                    } else {
                        // N:N direct routing
                        for (ch, val) in vals.iter().enumerate().take(ch_limit) {
                            let hw_ch = instance.output_channel + ch;
                            if hw_ch < out_channels {
                                let is_enabled = if hw_ch < GLOBAL_STATE.enabled_channels.len() {
                                    GLOBAL_STATE.enabled_channels[hw_ch].load(Ordering::Relaxed)
                                } else {
                                    false
                                };
                                let out_idx = frame * out_channels + hw_ch;
                                if is_enabled && out_idx < output.len() {
                                    output[out_idx] += val * current_vol;
                                }
                            }
                        }

                        // Mono file -> Stereo Out upmix for backwards compatibility
                        //
                        // 이 경로에는 두 가지 결함이 있었다.
                        // 1) enabled_channels 게이트를 건너뛰어서 Output Config
                        //    에서 닫은 채널로도 소리가 나갔다. 바로 위 N:N
                        //    라우팅은 게이트를 거치므로, 왼쪽은 음소거되고
                        //    오른쪽만 나오는 비대칭이 생겼다.
                        // 2) channel_spatial_gains를 오른쪽에만 곱했다. 이 배열은
                        //    궤적/공간 코드가 1.0에서 크게 벗어나게 변조하므로
                        //    (같은 파일 436~469행), 모노 소스를 스테레오 쌍으로
                        //    보내면 왼쪽은 원래 레벨, 오른쪽만 궤적 게인이 걸려
                        //    이미지가 한쪽으로 쏠리거나 오른쪽이 사라졌다.
                        //    믹스다운 전체에서 이 배열을 곱하는 곳은 여기뿐이었다.
                        // 이제 왼쪽과 같은 규칙(게이트 통과, 추가 게인 없음)을
                        // 적용해 좌우를 대칭으로 만든다.
                        if ch_limit == 1 && instance.output_stereo {
                            let hw_ch_r = instance.output_channel + 1;
                            if hw_ch_r < out_channels {
                                let is_enabled_r = if hw_ch_r < GLOBAL_STATE.enabled_channels.len() {
                                    GLOBAL_STATE.enabled_channels[hw_ch_r].load(Ordering::Relaxed)
                                } else {
                                    false
                                };
                                let out_idx_r = frame * out_channels + hw_ch_r;
                                if is_enabled_r && out_idx_r < output.len() {
                                    output[out_idx_r] += vals[0] * current_vol;
                                }
                            }
                        }
                    }

                    if instance.stream_receiver.is_some() && idx_i >= instance.stream_buffer.len() {
                        // Buffer is empty, stream is lagging. Don't advance cursor.
                    } else {
                        instance.cursor += step;
                    }
                } else {
                    instance.is_stopping = true;
                }
            }
        }
        
        // 재생 위치를 표에 알린다(재시작 복원·위치 조회). 원자 저장뿐이다.
        let cursors = &*crate::audio::playback_cursor::CURSOR_TABLE;
        for &i in active_instances.iter() {
            match &self.instances[i] {
                Some(inst) if inst.is_playing => cursors.publish(i, inst.instance_id, inst.position_seconds()),
                _ => cursors.clear(i),
            }
        }

        self.temp_vals = temp_vals;
        self.temp_active_instances = active_instances;

        // Apply Channel DSP
        let fs = self.sample_rate as f32;
        let dsp_limit = self.channel_dsp.len().min(out_channels);
        
        // ── 방별 베이스 매니지먼트 ─────────────────────────────────────
        // 서브 지정이 바뀌었으면, 적용량을 0까지 내린 블록 경계에서 새 라우팅 표로 바꾼다
        // (옛 서브 페이드아웃 → 교체 → 새 서브 페이드인).
        if self.bass_route_dirty && self.bm_mix == 0.0 {
            std::mem::swap(&mut self.bass_route, &mut self.bass_route_pending);
            std::mem::swap(&mut self.channel_is_sub, &mut self.channel_is_sub_pending);
            self.bass_route_dirty = false;
            // 새로 서브가 된 채널의 로우패스 상태를 비운다(다른 신호의 잔상이 섞이지 않게).
            // 적용량이 0인 지점이라 소리에는 드러나지 않는다. 고정 크기 대입만 한다(Law 1).
            for (ch, f) in self.lfe_track_lpfs.iter_mut().enumerate() {
                if self.channel_is_sub.get(ch).copied().unwrap_or(false) {
                    *f = crate::audio::crossover::LinkwitzRiley24::new();
                    f.set_crossover_freq(LFE_TRACK_LPF_HZ, fs);
                }
            }
        }
        let has_sub = self.channel_is_sub.iter().any(|&s| s);
        let bm_target = if has_sub && !self.bass_route_dirty { 1.0 } else { 0.0 };
        let bm_start = self.bm_mix;
        let bm_step = 1.0 / (BASS_MANAGEMENT_RAMP_S * fs);
        // Law 1: new()에서 사전 할당한 버퍼의 사용 구간만 비운다(서브가 된 채널 것만).
        let lfe_frames = frames.min(self.lfe_sub_mix.first().map_or(0, |b| b.len()));
        for (ch, buf) in self.lfe_sub_mix.iter_mut().enumerate() {
            if self.channel_is_sub.get(ch).copied().unwrap_or(false) {
                buf[..lfe_frames].fill(0.0);
            }
        }
        // 서브가 된 채널은 공간 효과(초기반사·리버브)를 서서히 뺀다(기존 스무딩).
        for (ch, dsp) in self.channel_dsp.iter_mut().enumerate() {
            dsp.mute_room_effects = self.channel_is_sub.get(ch).copied().unwrap_or(false);
        }

        for ch in 0..dsp_limit {
            let is_enabled = if ch < GLOBAL_STATE.enabled_channels.len() {
                GLOBAL_STATE.enabled_channels[ch].load(Ordering::Relaxed)
            } else {
                false
            };
            if !is_enabled { continue; }
            // 서브는 메인 채널을 다 처리해 합산 저역이 모인 **뒤에** 아래에서 처리한다.
            // 예전에는 합산 저역을 바이노럴 렌더와 서브 채널 DSP가 끝난 뒤에 더해서,
            // ① 서브의 딜레이·EQ·게인이 그 저역에 걸리지 않았고 ② 헤드폰 미리듣기에서
            // 메인이 잘라낸 저역이 통째로 사라졌다(소리가 날카롭게 들렸다).
            if self.channel_is_sub.get(ch).copied().unwrap_or(false) {
                continue;
            }
            // 이 채널의 저역을 받을 서브 = 자기 방의 서브(없으면 풀레인지).
            let route = self
                .bass_route
                .get(ch)
                .copied()
                .flatten()
                .filter(|&s| s < self.lfe_sub_mix.len());

            for frame in 0..frames {
                let sample_idx = frame * out_channels + ch;
                if sample_idx < output.len() {
                    let mut val = output[sample_idx];

                    // Bass Management Routing (LR24): 고역은 채널에 남기고 저역은 자기 방
                    // 서브로 보낸다. 크로스오버는 서브가 없을 때도 돌려서 필터 상태를 데워
                    // 둔다(빈 상태에서 켜면 과도 응답이 난다).
                    //
                    // 표준 순서대로 채널 DSP(딜레이·EQ·게인·극성·반사)보다 **앞에서** 가른다.
                    // 그 보정들은 이 스피커가 실제로 내는 소리(고역)에만 걸려야 하고, 서브로
                    // 넘어간 저역은 서브 채널의 보정만 받는다. 예전에는 DSP 뒤에서 갈라서
                    // 메인의 로우컷·쉘프·게인이 서브 저역에 한 번 더 걸렸고(실기: 9~13dB 작음),
                    // 극성이 뒤집힌 메인의 저역은 서브에서 다른 메인의 같은 저역을 지웠다.
                    if ch < self.crossovers.len() {
                        let (low_val, high_val) = self.crossovers[ch].split(val);
                        if let Some(sub) = route {
                            let m = bass_management_ramp_at(bm_start, bm_target, bm_step, frame);
                            if m > 0.0 {
                                val += (high_val - val) * m;
                                if frame < lfe_frames {
                                    self.lfe_sub_mix[sub][frame] += low_val * m;
                                }
                            }
                        }
                    }

                    output[sample_idx] = self.channel_dsp[ch].process(val, fs);
                }
            }
        }

        // 서브 출력 = LPF120(자기 신호) x (LFE +10dB 토글) + 자기 방 메인에서 넘어온 저역.
        // 그 합 전체가 서브 채널의 딜레이·EQ·게인·극성을 지난다(표준 서브 출력 체인).
        // +10dB 램프는 샘플 위치로 계산해서, 서브가 여럿이어도 모두 같은 게인을 쓴다.
        let boost_start = self.lfe_boost_gain;
        let boost_target = if self.lfe_boost_enabled { LFE_BOOST_LINEAR } else { 1.0 };
        let boost_step = (LFE_BOOST_LINEAR - 1.0) / (LFE_BOOST_RAMP_S * fs);
        for sub in 0..dsp_limit {
            if !self.channel_is_sub.get(sub).copied().unwrap_or(false) {
                continue;
            }
            let sub_enabled = sub < GLOBAL_STATE.enabled_channels.len()
                && GLOBAL_STATE.enabled_channels[sub].load(Ordering::Relaxed);
            if !sub_enabled {
                continue;
            }
            for frame in 0..frames {
                let idx = frame * out_channels + sub;
                if idx >= output.len() {
                    break;
                }
                let own = output[idx];
                let low = self.lfe_track_lpfs[sub].process_low(own);
                let m = bass_management_ramp_at(bm_start, bm_target, bm_step, frame);
                let boost = bass_management_ramp_at(boost_start, boost_target, boost_step, frame);
                // 자기 신호(.1 LFE 트랙): 로우패스 **이후**에 부스트한다.
                let track = own + (low * boost - own) * m;
                let folded = if frame < lfe_frames { self.lfe_sub_mix[sub][frame] } else { 0.0 };
                output[idx] = self.channel_dsp[sub].process(track + folded, fs);
            }
        }
        if frames > 0 {
            self.lfe_boost_gain =
                bass_management_ramp_at(boost_start, boost_target, boost_step, frames - 1);
        }

        // Apply Binaural Processing (De-interleaves, convolves, and re-interleaves to Ch0 & Ch1)
        self.binaural.process_interleaved(output, out_channels);

        // Apply Global Reverb to Ch0 and Ch1
        if out_channels >= 2 && self.reverb.mix > 0.0 {
            for frame in 0..frames {
                let idx_l = frame * out_channels + 0;
                let idx_r = frame * out_channels + 1;
                if idx_r < output.len() {
                    let (rl, rr) = self.reverb.process_stereo(output[idx_l], output[idx_r]);
                    output[idx_l] = rl;
                    output[idx_r] = rr;
                }
            }
        }

        if frames > 0 {
            self.bm_mix = bass_management_ramp_at(bm_start, bm_target, bm_step, frames - 1);
        }

        // Compute VU levels (Peak per channel) and apply soft clipping
        let headroom_gain = 10.0f32.powf(self.master_headroom_db / 20.0);
        
        for ch in 0..out_channels {
            let mut peak: f32 = 0.0;
            for frame in 0..frames {
                let sample_idx = frame * out_channels + ch;
                if sample_idx < output.len() {
                    let mut val = output[sample_idx];

                    // Apply Master Headroom Padding
                    val *= headroom_gain;

                    // Output Peak Limiter Guard
                    if self.peak_limiter_enabled && ch < self.limiters.len() {
                        val = self.limiters[ch].process(val);
                    } else {
                        // Fallback absolute hard clamp if limiter is disabled
                        if val > 1.0 {
                            val = 0.99;
                        } else if val < -1.0 {
                            val = -0.99;
                        }
                    }
                    val *= self.startup_ramp.block_gain(frame);
                    if self.master_mute {
                        val = 0.0;
                    }
                    if GLOBAL_STATE.is_failover_mode.load(Ordering::Relaxed) {
                        val *= 0.01; // -40dB safety pad
                    }
                    output[sample_idx] = val;

                    let abs_val = val.abs();
                    if abs_val > peak {
                        peak = abs_val;
                    }
                }
            }

            GLOBAL_STATE.vu_levels[ch].store(peak.to_bits(), Ordering::Relaxed);
        }

        // 부팅 뮤트 램프는 블록당 한 번만 전진시킨다(채널 루프 안에서 전진하면
        // 채널 수만큼 빨라진다 - block_gain 문서 참고).
        self.startup_ramp.advance_block(frames);

        // Store first two channels LUFS to global state (assume stereo master)
        if out_channels > 0 && !self.limiters.is_empty() {
            let lufs = self.limiters[0].short_term_lufs;
            GLOBAL_STATE.current_master_lufs.store(lufs.to_bits(), Ordering::Relaxed);

            // 마스터 게인 리덕션(dB): 활성 리미터들 중 최댓값을 대표값으로 사용.
            let mut max_gr_db = 0.0f32;
            for limiter in self.limiters.iter().take(out_channels) {
                let gr = limiter.current_gain_reduction_db();
                if gr > max_gr_db {
                    max_gr_db = gr;
                }
            }
            GLOBAL_STATE.current_gain_reduction_db.store(max_gr_db.to_bits(), Ordering::Relaxed);
        }
        
        // Send to Analysis Thread (Lock-free)
        if let Some(tx) = &mut self.analysis_tx {
            let available = tx.slots();
            if available >= output.len() {
                if let Ok(mut chunk) = tx.write_chunk(output.len()) {
                    let (slice1, slice2) = chunk.as_mut_slices();
                    let len1 = slice1.len();
                    slice1.copy_from_slice(&output[..len1]);
                    if !slice2.is_empty() {
                        slice2.copy_from_slice(&output[len1..]);
                    }
                    chunk.commit_all();
                }
            }
        }

        // Remove stopped instances by moving to GC thread (heap-free drop)
        // Try to flush local_recycle to gc_sender if possible
        while !self.local_recycle.is_empty() {
            if let Some(chunk) = self.local_recycle.pop() {
                if let Err(e) = self.buf_gc_tx.try_send(chunk) {
                    self.local_recycle.extend(std::iter::once(e.into_inner()));
                    break; // Channel is still full
                }
            }
        }

        for slot in self.instances.iter_mut() {
            if let Some(inst) = slot {
                if !inst.is_playing {
                    if let Some(old) = slot.take() {
                        // 정리 채널이 가득 찼으면 버리지 않고 자리에 되돌려 다음 블록에 다시 보낸다. 여기서 버리면
                        // 메모리 해제와(스트리밍이면) 디코더 스레드 join이 콜백 안에서 일어난다.
                        // 받는 쪽이 끝났으면(정리 스레드 없음 — 시험용 믹서 등) 되돌려도 영영 못 보내므로 예전처럼
                        // 여기서 버린다.
                        if let Err(crossbeam_channel::TrySendError::Full(back)) = self.gc_sender.try_send(old) {
                            *slot = Some(back);
                        }
                    }
                }
            }
        }
    }

    /// 바이노럴 렌더러에 채널별 기준 방위각을 넘긴다. `channel_positions`나
    /// `room_zones`가 바뀔 때(`UpdateSpatialConfig` 처리 중)만 호출하면
    /// 된다 — 오디오 콜백마다 부를 필요는 없다(binaural 쪽이 dirty 플래그로
    /// 스스로 재보간 시점을 판단한다).
    ///
    /// 좌표계·공식은 `docs/02_Planning_and_Specs/BINAURAL_SPATIAL_RENDERING_SPEC.md`
    /// §2.1을 따른다: +Y=정면(0°), +X=오른쪽(+90°). 리스너는 바인딩된
    /// RoomZone의 수평 중심이다(고도는 방위각 계산에 안 쓰므로 ear_level은
    /// 참조하지 않는다).
    ///
    /// 룸 바인딩 판정은 pan_deg 피벗 계산(위 process() 내부)이 이미 하는
    /// point-in-room 루프와 로직이 겹친다. 지금은 그 코드를 공유 헬퍼로
    /// 뽑지 않고 여기 별도로 작게 둔다 — 기존에 검증된 pan_deg/초기반사음
    /// 경로를 건드리지 않는 것이 이번 변경의 위험을 줄인다는 판단이다
    /// (스펙 문서의 "권장"일 뿐 필수 요구는 아니다). 세 번째 자리가 생겼으니
    /// 다음에 손댈 때는 공유 헬퍼로 통합을 고려할 것.
    /// 채널의 초기반사 탭을 DSP 스무딩 타겟으로 옮기고, 연출용 정규화 배율을 갱신한다.
    /// 커맨드 처리(UpdateSpatialConfig)에서 호출된다 — 고정 배열 대입만 하고 힙 할당은 없다.
    pub fn apply_early_reflection_taps(
        &mut self,
        channel: usize,
        taps: &[crate::audio::acoustic::EarlyReflectionTap;
             crate::audio::acoustic::MAX_EARLY_REFLECTION_TAPS],
    ) {
        if channel >= self.channel_dsp.len() {
            return;
        }
        for i in 0..crate::audio::acoustic::MAX_EARLY_REFLECTION_TAPS {
            self.channel_dsp[channel].taps[i].target_delay_ms = taps[i].delay_ms;
            self.channel_dsp[channel].taps[i].target_gain = taps[i].gain;
            self.channel_dsp[channel].taps[i].target_hf_shelf_db = taps[i].hf_shelf_db;
        }
        if channel < self.channel_er_effect_scale.len() {
            self.channel_er_effect_scale[channel] =
                crate::audio::acoustic::early_reflection_effect_scale(taps);
        }
        self.refresh_early_ref_mix(channel);
    }

    /// 사용자가 돌린 연출용 초기반사 믹스(0~1)를 기록한다.
    pub fn set_channel_early_ref_mix(&mut self, channel: usize, mix: f32) {
        if channel < self.channel_er_mix.len() {
            self.channel_er_mix[channel] = mix.clamp(0.0, 1.0);
        }
        self.refresh_early_ref_mix(channel);
    }

    /// 실제 적용량 = 방 시뮬레이션 몫 + 연출용 몫.
    ///
    /// 방 시뮬레이션(그 방이 만들 반사를 물리 그대로)은 **헤드폰 미리듣기(바이노럴)에서만**
    /// 건다. 현장에서는 진짜 벽이 같은 반사를 만들기 때문에 겹치면 반사가 두 번 들어간다.
    /// 연출용 몫은 방과 무관하게 슬라이더 감각이 같도록 정규화해서 더한다.
    /// 값 변경은 dsp.rs에서 샘플 단위로 스무딩된다(Law 3).
    pub fn refresh_early_ref_mix(&mut self, channel: usize) {
        if channel >= self.channel_dsp.len() {
            return;
        }
        // 방 시뮬레이션도 연출용과 같은 방식으로 정규화한다(물리 그대로 쓰면 너무 세다 —
        // acoustic::EARLY_REFLECTION_ROOM_SIM_TARGET_DB 주석 참고).
        let room_sim = if self.binaural.enabled {
            crate::audio::acoustic::room_sim_scale_from_effect_scale(
                self.channel_er_effect_scale.get(channel).copied().unwrap_or(0.0),
            )
        } else {
            0.0
        };
        let effect = self.channel_er_mix.get(channel).copied().unwrap_or(0.0)
            * self.channel_er_effect_scale.get(channel).copied().unwrap_or(0.0);
        self.channel_dsp[channel].target_early_ref_mix = room_sim + effect;
    }

    /// 바이노럴 on/off처럼 전 채널에 영향을 주는 변경 뒤에 부른다.
    pub fn refresh_all_early_ref_mixes(&mut self) {
        for ch in 0..self.channel_dsp.len() {
            self.refresh_early_ref_mix(ch);
        }
    }

    /// 방별 베이스 매니지먼트 라우팅을 바꾼다(엔진은 UpdateSpatialConfig로 받는다).
    ///
    /// `route[ch]` = 그 채널의 저역을 받을 서브(audio::bass_route::compute_bass_route),
    /// `is_sub[ch]` = 그 채널이 서브우퍼인가. 표가 지금과 같으면 아무것도 바꾸지 않는다
    /// (스피커를 드래그하면 초당 수십 번 들어온다). 다르면 대기 표로 두고, process()가
    /// 적용량을 0까지 내린 뒤 교체한다(딸깍 방지).
    ///
    /// 돌려주는 옛 대기 표는 오디오 스레드에서 해제하지 말고 GC 스레드로 보낸다(Law 1).
    pub fn set_bass_routing(
        &mut self,
        route: Vec<Option<usize>>,
        is_sub: Vec<bool>,
    ) -> (Vec<Option<usize>>, Vec<bool>) {
        self.bass_route_dirty = route != self.bass_route || is_sub != self.channel_is_sub;
        let old_route = std::mem::replace(&mut self.bass_route_pending, route);
        let old_is_sub = std::mem::replace(&mut self.channel_is_sub_pending, is_sub);
        (old_route, old_is_sub)
    }

    /// 헤드폰 미리듣기에 들려줄 방을 정한다(지금 보고 있는 방).
    ///
    /// 설계는 방 단위로 하므로, 그 방의 스피커만 헤드폰에 섞여야 한다. 방을 아직
    /// 만들지 않았거나 지정이 없으면(None) 예전처럼 전체 채널을 렌더링한다.
    pub fn set_binaural_room(&mut self, room_id: Option<u32>) {
        self.binaural_room = room_id;
        self.refresh_binaural_channel_mask();
    }

    /// 채널 마스크를 현재 방 지정과 채널별 방 ID로 다시 계산한다.
    /// 방 지정이나 스피커 배치가 바뀔 때만 부르면 된다(Law 1: 사전 할당 버퍼에 쓰기만).
    ///
    /// 서브우퍼도 같은 규칙을 따른다. 베이스 매니지먼트가 방별이라, 보고 있는 방의 저역은
    /// 그 방 안의 서브로 모인다(그 서브는 방 규칙으로 들린다). 다른 방 서브는 다른 방의
    /// 저역을 담고 있으므로 넣지 않는다.
    pub fn refresh_binaural_channel_mask(&mut self) {
        let room = self.binaural_room;
        let room_ids = &self.channel_room_ids;
        let positions = &self.channel_positions;
        // 스피커를 하나라도 배치했으면, 스피커가 없는 채널(설계에 없는 채널 — 예: 6채널 장비의
        // CH3~6)은 헤드폰에 섞지 않는다. 예전에는 정면에서 어느 방에서나 들려서 헷갈렸다.
        // 아직 하나도 배치하지 않았으면 예전처럼 전부 들려준다(헤드폰이 무음이 되지 않게).
        let any_placed = positions.iter().any(|p| p.is_some());
        let mask = self.binaural.channel_enabled_mut();
        for (ch, slot) in mask.iter_mut().enumerate() {
            let placed = positions.get(ch).is_some_and(|p| p.is_some());
            if any_placed && !placed {
                *slot = false;
                continue;
            }
            *slot = match (room, room_ids.get(ch).copied().flatten()) {
                // 방 지정이 없으면 전체 채널.
                (None, _) => true,
                // 방이 지정되지 않은 예전 스피커는 3D 화면과 같은 규칙으로 어느 방에서나
                // 들린다(dynamic_3d_room.dart: roomId == null이면 모든 방에 표시).
                (Some(_), None) => true,
                (Some(target), Some(id)) => id == target,
            };
        }
    }

    pub fn recalculate_binaural_channel_azimuths(&mut self) {
        // Law 1: 임시 Vec을 만들지 않는다. 바이노럴이 들고 있는 사전 할당
        // 버퍼에 바로 쓴다(channel_base_azimuth_mut()). UpdateSpatialConfig
        // 핸들러는 실제 크래시 스택으로 오디오 스레드 위임이 확인된 코드
        // 경로다.
        let room_zones = &self.room_zones;
        let channel_positions = &self.channel_positions;
        let channel_room_ids = &self.channel_room_ids;
        let explicit_listener = self.listener_position.as_ref().map(|p| (p.x, p.y));
        let explicit_listener_z = self.listener_position.as_ref().map(|p| p.z);
        let (azimuths, elevations) = self.binaural.channel_base_angles_mut();

        // RoomZone이 정의되지 않은 경우(기본 상태)에도 방위각이 나와야 한다.
        // 예전에는 바인딩된 zone이 없으면 전부 0°(정면)로 처리해서, RoomZone을
        // 만들지 않은 사용자에게는 스피커를 아무리 옮겨도 바이노럴이 전혀
        // 반응하지 않았다(실기 보고: "스피커 위치를 바꿨는데 반영이 안 된다").
        // 방위각은 리스너 기준점만 있으면 되고 벽 경계는 필요 없으므로,
        // zone이 없으면 **배치된 스피커들의 무게중심**을 리스너로 삼는다.
        // (초기반사음은 벽이 있어야 계산되므로 여전히 zone이 필요하다.)
        let fallback_listener = {
            let mut sx = 0.0f32;
            let mut sy = 0.0f32;
            let mut count = 0.0f32;
            for p in channel_positions.iter().flatten() {
                sx += p.x;
                sy += p.y;
                count += 1.0;
            }
            if count > 0.0 {
                Some((sx / count, sy / count))
            } else {
                None
            }
        };

        let n = azimuths.len().min(elevations.len()).min(channel_positions.len());
        for ch in 0..n {
            let Some(pos) = &channel_positions[ch] else {
                azimuths[ch] = 0.0; // 좌표 없음 -> 정면 취급
                elevations[ch] = 0.0;
                continue;
            };

            let bound_zone = crate::audio::acoustic::bind_channel_zone(
                room_zones,
                channel_room_ids.get(ch).copied().flatten(),
                pos,
            );

            // 기준점 우선순위:
            // 1. 프론트엔드가 보낸 리스너 좌표(3D 룸의 마네킹 위치 = 방 중심).
            //    실제로 소리를 듣는 지점이므로 이게 가장 정확하다.
            // 2. 스피커가 바인딩된 RoomZone의 중심.
            // 3. 배치된 스피커들의 무게중심(최후 폴백). 스피커가 일직선으로
            //    놓이면 무게중심도 그 선 위에 놓여 모든 채널이 정확히 ±90°
            //    (하드 좌우)가 되는 퇴화가 생기므로, 1번이 있으면 반드시
            //    1번을 쓴다.
            let (listener_x, listener_y) = match explicit_listener {
                Some(c) => c,
                None => match bound_zone {
                    Some(zone) => (
                        (zone.boundary_min.x + zone.boundary_max.x) * 0.5,
                        (zone.boundary_min.y + zone.boundary_max.y) * 0.5,
                    ),
                    None => match fallback_listener {
                        Some(c) => c,
                        None => {
                            azimuths[ch] = 0.0;
                            elevations[ch] = 0.0;
                            continue;
                        }
                    },
                },
            };

            let dx = pos.x - listener_x;
            let dy = pos.y - listener_y;

            // 부호를 뒤집지 않는다. 두 가지 사실이 서로를 상쇄한다.
            //
            // 1) SOFA 조회에 들어가는 azimuth_deg는 양수=왼쪽 우세다
            //    (binaural_numeric_check.rs로 실측 확인, 수학 교과서의
            //    "0°=정면, +90°=오른쪽" 관례와 반대).
            // 2) 실제 3D 룸(assets/3d_simulator/studio_engine.html)에서
            //    Dart의 채널 x좌표는 `posX = sp.x - room.width/2`로 Three.js
            //    world X에 그대로 들어간다(반전 없음). 사용자가 "정면"이라
            //    부르는 카메라(마네킹 뒤통수 너머로 마네킹 눈이 보는 방향을
            //    보는 각도 = 'Back View' 프리셋, studio_engine.html:607-610,
            //    camera가 -Z에서 +Z를 바라봄)에서는 forward×up 벡터 계산상
            //    화면 오른쪽이 world -X다. 즉 dx>0(월드 +X)는 이 카메라
            //    기준 화면 왼쪽, dx<0이 화면 오른쪽이다.
            //
            // 두 사실을 합치면: dx<0(화면 오른쪽) -> dx.atan2(dy)가 음수
            // azimuth를 만들고 -> 음수 azimuth는 오른쪽 귀 우세. 정확히
            // 맞다. 처음에 -dx로 뒤집었던 건 1번만 확인하고 2번(실제 룸
            // 좌표 매핑)을 검증 없이 "월드 +X=화면 오른쪽"이라고 가정해서
            // 생긴 회귀였다 — 실기(스피커 레이아웃 + 헤드폰)에서 "Ch1이
            // 오른쪽에 있는데 왼쪽에서 들린다"는 사용자 보고로 발견했다.
            azimuths[ch] = dx.atan2(dy).to_degrees();
            // 고도각: 청취 지점(귀 높이) 기준 위(+)·아래(−). 예전에는 모든 스피커를 귀 높이로
            // 렌더링해서 천장 가까이 매단 스피커도 앞에서 들렸다. 귀 높이를 모르면 0.
            let listener_z = explicit_listener_z.or_else(|| bound_zone.map(|z| z.ear_level));
            elevations[ch] = match listener_z {
                Some(lz) => (pos.z - lz).atan2((dx * dx + dy * dy).sqrt()).to_degrees(),
                None => 0.0,
            };
        }

        self.binaural.mark_base_azimuth_dirty();
        self.recalculate_binaural_propagation();
    }

    /// 헤드폰 미리듣기의 채널별 전파 흉내(스피커 → 청취 지점: 지연·거리 감쇠)를 다시
    /// 계산한다. 현장에서 공기가 만드는 것과 같은 지연·감쇠라, 자동 튜닝의 시간 정렬
    /// 딜레이·거리 게인이 헤드폰에서도 현장처럼 상쇄된다(binaural.rs Propagation 참고).
    ///
    /// 청취 지점은 초기반사·공기흡음과 같은 규칙(acoustic::listening_point)을 쓴다.
    /// 오디오 스레드(UpdateSpatialConfig)에서 불린다 — 계산과 고정 슬롯 쓰기만 한다(Law 1).
    fn recalculate_binaural_propagation(&mut self) {
        for ch in 0..self.binaural.channel_count() {
            let Some(pos) = self.channel_positions.get(ch).and_then(|p| p.as_ref()) else {
                self.binaural.clear_channel_propagation(ch);
                continue;
            };
            let zone = crate::audio::acoustic::bind_channel_zone(
                &self.room_zones,
                self.channel_room_ids.get(ch).copied().flatten(),
                pos,
            );
            match crate::audio::acoustic::gain_reference_distance(zone, self.listener_position.as_ref()) {
                Some(reference) => {
                    let lp = crate::audio::acoustic::listening_point(
                        self.listener_position.as_ref(),
                        zone,
                        pos,
                    );
                    let distance = crate::audio::acoustic::distance_3d(pos, &lp);
                    self.binaural.set_channel_propagation(ch, distance, reference);
                }
                None => self.binaural.clear_channel_propagation(ch),
            }
        }
    }

    pub fn recalculate_spatial_dsp(&mut self) {
        if let Ok(config_guard) = crate::core::state::GLOBAL_STATE.config.try_read() {
            if let Some(config) = config_guard.as_ref() {
                let max_ch = self.channel_dsp.len();
                let max_pos = self.channel_positions.len();

                // 1. Collect base delay and EQs from config
                let mut base_delays = std::mem::take(&mut self.temp_base_delays);
                let mut base_eqs = std::mem::take(&mut self.temp_base_eqs);
                let mut channel_dists = std::mem::take(&mut self.temp_channel_dists);
                
                // Ensure capacities match
                if base_delays.len() < max_ch { base_delays.resize(max_ch, 0.0); }
                if base_eqs.len() < max_ch { base_eqs.resize(max_ch, Vec::new()); }
                if channel_dists.len() < max_pos { channel_dists.resize(max_pos, 0.0); }
                
                for i in 0..max_ch {
                    base_delays[i] = 0.0;
                    base_eqs[i].clear();
                }
                for i in 0..max_pos {
                    channel_dists[i] = 0.0;
                }
                
                let mut apply_config = |ch_key: u32, setting: &crate::common::config::ChannelSetting, ch_offset: usize, count: usize| {
                    if ch_key > 0 {
                        let ch_idx_base = (ch_key - 1) as usize + ch_offset;
                        for i in 0..count {
                            let ch_idx = ch_idx_base + i;
                            if ch_idx < max_ch {
                                base_delays[ch_idx] = setting.delay_ms;
                                base_eqs[ch_idx].clone_from(&setting.eq_bands);
                            }
                        }
                    }
                };

                for (&ch_key, setting) in &config.mono_configs { apply_config(ch_key, setting, 0, 1); }
                for (&ch_key, setting) in &config.stereo_configs { apply_config(ch_key, setting, 0, 2); }
                for (&ch_key, setting) in &config.multi_configs { apply_config(ch_key, setting, 0, 6); }

                // 2. Find target pos
                let target_pos = if let Some(traj) = &self.trajectory {
                    Some(traj.current_position.clone())
                } else {
                    None
                };

                // 3. Pre-calculate max_dist for time alignment
                let mut max_dist = 0.0_f32;
                
                let target_room_id = if let Some(traj) = &self.trajectory {
                    traj.target_room_zone_id.as_ref().map(|s| s.parse::<u32>().unwrap_or_else(|_| crate::common::utils::hash_id(s)))
                } else {
                    None
                };

                for ch_idx in 0..self.channel_positions.len() {
                    if let Some(pos) = &self.channel_positions[ch_idx] {
                        let bound_room_id = crate::audio::acoustic::bind_channel_zone(
                            &self.room_zones,
                            self.channel_room_ids.get(ch_idx).copied().flatten(),
                            pos,
                        )
                        .map(|z| z.room_id);

                        let in_target_room = match target_room_id {
                            Some(target_id) => bound_room_id == Some(target_id),
                            None => true,
                        };

                        if in_target_room {
                            let t_pos = if let Some(tp) = &target_pos {
                                tp.clone()
                            } else {
                                let zone = bound_room_id
                                    .and_then(|rid| self.room_zones.iter().find(|z| z.room_id == rid));
                                crate::audio::acoustic::listening_point(
                                    self.listener_position.as_ref(),
                                    zone,
                                    pos,
                                )
                            };
                            
                            let dist = crate::audio::acoustic::distance_3d(pos, &t_pos);
                            channel_dists[ch_idx] = dist;
                            if dist > max_dist {
                                max_dist = dist;
                            }
                        } else {
                            channel_dists[ch_idx] = -1.0;
                        }
                    }
                }

                // 4. Apply Time Alignment & Dynamic Off-axis EQ
                // channel_positions는 UpdateSpatialConfig 커맨드로 프론트엔드가 통째로 교체하므로
                // channel_dsp(가상 채널 고정 길이)와 길이가 다를 수 있음 -> max_pos로 하한 클램프
                for ch_idx in 0..max_ch.min(max_pos) {
                    let dist = channel_dists[ch_idx];
                    if dist < 0.0 {
                        continue; // Not participating in spatial DSP (isolated)
                    }

                    if let Some(pos) = &self.channel_positions[ch_idx] {
                        let matched_zone = crate::audio::acoustic::bind_channel_zone(
                            &self.room_zones,
                            self.channel_room_ids.get(ch_idx).copied().flatten(),
                            pos,
                        );

                        let t_pos = if let Some(tp) = &target_pos {
                            tp.clone()
                        } else {
                            crate::audio::acoustic::listening_point(
                                self.listener_position.as_ref(),
                                matched_zone,
                                pos,
                            )
                        };
                        
                        // Phase 2: Time Alignment Formula
                        let delay_seconds = (max_dist - dist) / crate::audio::acoustic::SPEED_OF_SOUND_M_S;
                        let acoustic_delay_ms = delay_seconds * 1000.0;
                        let zone_delay = matched_zone.map(|z| z.boundary_delay_ms).unwrap_or(0.0);
                        
                        let base_delay = base_delays[ch_idx];
                        // 채널별 딜레이/EQ의 소유자는 Dart의 음향 동기화
                        // (acoustic_sync_provider)다. 여기서도 같은 필드를
                        // 계산해 쓰면 두 시스템이 서로 덮어써서, 나중에 도착한
                        // 명령에 따라 값이 요동친다(실기 보고: "스피커를 누르면
                        // EQ 계열 값이 미묘하게 바뀐다"). 계산 자체는 남겨두되
                        // (아래 로그/후속 작업 참고) 기록하지 않는다.
                        //
                        // 공기 흡음도 여기(채널 DSP = 실제 스피커 출력)에 걸지 않는다. 현장에서는
                        // 진짜 공기가 흡음하므로 두 번 깎인다. 헤드폰 미리듣기에서만 바이노럴
                        // 전파 흉내가 건다(recalculate_binaural_propagation).
                        let _ = base_delay + acoustic_delay_ms + zone_delay;

                        // Phase 2: Dispersion Angle off-axis EQ roll-off
                        let mut dynamic_eq = None;
                        if pos.dispersion_angle > 0.0 {
                            let forward_vec = crate::audio::acoustic::calculate_forward_vector(pos.pitch_tilt, pos.yaw_rotation);
                            let to_target_vec = crate::audio::acoustic::vector_from_to(pos, &t_pos);
                            
                            let f_norm = crate::audio::acoustic::normalize(&forward_vec);
                            let t_norm = crate::audio::acoustic::normalize(&to_target_vec);
                            let theta = crate::audio::acoustic::angle_between_vectors(&f_norm, &t_norm);
                            
                            let half_dispersion = pos.dispersion_angle / 2.0;
                            if theta > half_dispersion {
                                let excess_angle = theta - half_dispersion;
                                let roll_off_gain = - (excess_angle * 0.5).min(24.0); // max -24dB attenuation
                                
                                dynamic_eq = Some(crate::common::config::EqBand {
                                    enabled: true,
                                    freq: 4000.0,
                                    gain: roll_off_gain,
                                    q_factor: 0.707,
                                    filter_type: crate::common::config::EqType::HighShelf,
                                    slope_db_per_oct: 12,
                                });
                            }
                        }

                        // Apply Base EQ, Boundary EQ, and Dynamic EQ
                        let final_eqs = &mut base_eqs[ch_idx];
                        
                        if let Some(zone) = matched_zone {
                            for b_eq in &zone.boundary_eq_bands {
                                let mut already_exists = false;
                                for band in final_eqs.iter_mut() {
                                    if band.enabled && (band.freq - b_eq.freq).abs() < 1.0 && band.filter_type == b_eq.filter_type {
                                        already_exists = true;
                                        band.gain = b_eq.gain;
                                        break;
                                    }
                                }
                                if !already_exists {
                                    final_eqs.push(b_eq.clone());
                                }
                            }
                        }

                        if let Some(dyn_eq) = dynamic_eq {
                            let mut already_exists = false;
                            for band in final_eqs.iter_mut() {
                                if band.enabled && (band.freq - dyn_eq.freq).abs() < 1.0 && band.filter_type == dyn_eq.filter_type {
                                    already_exists = true;
                                    band.gain = dyn_eq.gain;
                                    break;
                                }
                            }
                            if !already_exists {
                                final_eqs.push(dyn_eq);
                            }
                        }

                        // Make sure we don't exceed MAX_EQ_BANDS by truncating
                        if final_eqs.len() > crate::audio::dsp::dsp_utils::MAX_EQ_BANDS {
                            final_eqs.truncate(crate::audio::dsp::dsp_utils::MAX_EQ_BANDS);
                        }

                        // 위와 같은 이유로 EQ도 여기서 기록하지 않는다.
                        // 오프액시스(분산각) 롤오프는 Dart 쪽 음향 동기화로
                        // 옮겨서 FX 패널에 보이고 엔지니어가 수정할 수 있게 했다.
                        let _ = &final_eqs;
                    }
                }
                
                self.temp_base_delays = base_delays;
                self.temp_base_eqs = base_eqs;
                self.temp_channel_dists = channel_dists;
            }
        }
    }
}
