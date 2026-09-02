use crate::audio::player::SoundData;
use std::sync::Arc;

pub enum AudioCommand {
    PlayTrack {
        instance_id: u64,
        room_id: u32,
        track_id: u32,
        track_id_str: String,
        data: Option<Arc<SoundData>>,
        streamer: Option<crate::audio::streaming::DiskStreamer>,
        stream_sample_rate: u32,
        stream_channels: u16,
        is_loop: bool,
        volume: f32,
        room_volume: f32,
        output_channel: usize,
        output_stereo: bool,
        current_position: Option<crate::common::config::Point3D>,
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
