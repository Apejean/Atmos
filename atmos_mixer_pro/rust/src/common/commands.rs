use crate::audio::player::SoundInstance;

pub enum AudioCommand {
    /// 재생할 인스턴스는 오디오 스레드 밖(api_play_track, FRB 워커 스레드)에서
    /// 미리 생성해 보낸다. 오디오 스레드는 풀 슬롯에 옮겨 담기만 하므로
    /// 콜백 내 힙 할당이 발생하지 않는다(Law 1).
    PlayTrack {
        instance: Box<SoundInstance>,
        room_volume: f32,
    },
    StopTrack {
        room_id: u32,
        track_id: u32,
    },
    StopAll,
    SetBinauralEnabled {
        enabled: bool,
    },
    SetReverbParams {
        mix: f32,
        decay: f32,
    },
    SetMasterMute {
        muted: bool,
    },
    ApplyAllChannelTunings {
        tunings: Vec<(usize, f32, Vec<crate::common::config::EqBand>, bool, f32)>,
    },
    SetMasterVolume {
        room_id: u32,
        volume: f32,
    },
    SetTrackVolume {
        room_id: u32,
        track_id: u32,
        volume: f32,
    },
    SetTrackOutput {
        room_id: u32,
        track_id: u32,
        output_channel: usize,
        output_stereo: bool,
    },
    ClearRoom {
        room_id: u32,
    },
    SetChannelDelay {
        channel: usize,
        delay_ms: f32,
    },
    SetSpatialReverb {
        is_enabled: bool,
        room_size: f32,
        decay_time: f32,
        pre_delay_ms: f32,
        damp: f32,
        density: f32,
        dry_wet: f32,
    },
    SetChannelSpatialReverb {
        channel: usize,
        is_enabled: bool,
        room_size: f32,
        decay_time: f32,
        pre_delay_ms: f32,
        damp: f32,
        density: f32,
        dry_wet: f32,
    },
    SetChannelReverbSend {
        channel: usize,
        send: f32,
    },
    /// LFE +10dB 토글. 서브 채널 자기 신호(.1 LFE 트랙)를 120Hz 로우패스 이후에 +10dB.
    SetLfeBoostEnabled {
        enabled: bool,
    },
    // 서브우퍼 지정(베이스 매니지먼트)은 방별이라 스피커 속성으로 UpdateSpatialConfig에
    // 실려 온다(channel_is_sub, bass_route). 예전의 전역 SetBassManagementEnabled /
    // SetLfeChannel은 서브가 하나뿐이라 다른 방 저역까지 모아서 없앴다.
    SetCrossoverFrequency {
        freq: f32,
    },
    SetChannelEq {
        channel: usize,
        bands: Vec<crate::common::config::EqBand>,
    },
    ApplyChannelTuning {
        channel: usize,
        delay_ms: f32,
        eq_bands: Vec<crate::common::config::EqBand>,
        phase_invert: bool,
        gain_db: f32,
    },
    UpdateSpatialConfig {
        /// 리스너(마네킹) 기준점. 방위각 계산의 기준이며, RoomZone이 없어도
        /// 스피커 위치가 반영되게 한다. 없으면 엔진이 폴백한다.
        listener_position: Option<crate::common::config::Point3D>,
        channel_positions: Vec<Option<crate::common::config::Point3D>>,
        /// 채널별 스피커가 속한 방 ID(RoomZone.room_id). 없으면 좌표로 찾는다
        /// (acoustic::bind_channel_zone 참고).
        channel_room_ids: Vec<Option<u32>>,
        /// 지금 화면에서 보고 있는 방. 헤드폰 미리듣기(바이노럴)를 이 방 기준으로
        /// 계산한다. None이면 전체 채널을 렌더링한다.
        active_room_id: Option<u32>,
        room_zones: Vec<crate::common::config::RoomZone>,
        trajectory: Option<crate::common::config::Trajectory>,
        track_positions: std::collections::HashMap<String, crate::common::config::Point3D>,
        // 채널별 초기반사음(1차 반사) 탭 6슬롯. len == channel_positions.len().
        // api_update_spatial_config_json()(비-오디오 스레드)에서 미리 계산되어 실려온다.
        early_reflection_taps: Vec<[crate::audio::acoustic::EarlyReflectionTap; 6]>,
        /// 채널별 서브우퍼 지정(스피커 인스펙터의 Set as LFE Subwoofer).
        channel_is_sub: Vec<bool>,
        /// 채널별 저역을 보낼 같은 방 서브 채널(audio::bass_route::compute_bass_route).
        /// 비-오디오 스레드에서 미리 계산해 오디오 스레드는 표를 바꿔 끼우기만 한다.
        bass_route: Vec<Option<usize>>,
        /// 채널별 현장 물리 밴드(헤드폰 미리듣기 전용, 슬롯 고정). Dart position_eq.dart의
        /// computeFieldPhysicsBands가 자동 EQ와 같은 모델로 계산해 보낸다.
        channel_sim_bands:
            Vec<[crate::common::config::EqBand; crate::audio::binaural::MAX_SIM_BANDS]>,
    },
    SetChannelPanDeg {
        channel: usize,
        pan_deg: f32,
    },
    SetChannelEarlyRefMix {
        channel: usize,
        mix: f32,
    },
    UpdateTrajectoryPosition {
        position: crate::common::config::Point3D,
    },
    UpdateSingleBandEq {
        channel: usize,
        band: usize,
        freq: f32,
        gain_db: f32,
        q_factor: f32,
        filter_type_idx: u8,
    },
    UpdateSoundSourcePosition {
        sound_id: String,
        x: f32,
        y: f32,
        z: f32,
    },
    ApplyGlobalTuning {
        master_headroom_db: f32,
        peak_limiter_enabled: bool,
    },
}
