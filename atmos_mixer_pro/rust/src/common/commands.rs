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
    PlayTestNoise {
        channel: u32,
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
    SetBassManagementEnabled {
        enabled: bool,
    },
    SetCrossoverFrequency {
        freq: f32,
    },
    SetLfeChannel {
        channel: Option<usize>,
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
        channel_positions: Vec<Option<crate::common::config::Point3D>>,
        room_zones: Vec<crate::common::config::RoomZone>,
        trajectory: Option<crate::common::config::Trajectory>,
        track_positions: std::collections::HashMap<String, crate::common::config::Point3D>,
        // 채널별 초기반사음(1차 반사) 탭 6슬롯. len == channel_positions.len().
        // api_update_spatial_config_json()(비-오디오 스레드)에서 미리 계산되어 실려온다.
        early_reflection_taps: Vec<[crate::audio::acoustic::EarlyReflectionTap; 6]>,
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
