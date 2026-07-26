use crate::audio::player::SoundData;
use std::sync::Arc;

#[derive(Clone)]
pub enum AudioCommand {
    PlayTrack {
        instance_id: u64,
        room_id: u32,
        track_id: u32,
        track_id_str: String,
        data: Option<Arc<SoundData>>,
        stream_receiver: Option<crossbeam_channel::Receiver<Vec<f32>>>,
        stream_sample_rate: u32,
        stream_channels: u16,
        is_loop: bool,
        volume: f32,
        output_channel: usize,
        output_stereo: bool,
    },
    StopTrack {
        room_id: u32,
        track_id: u32,
    },
    StopAll,
    SetMasterMute {
        muted: bool,
    },
    PlayTestNoise {
        channel: u32,
    },
    ApplyAllChannelTunings {
        tunings: Vec<(usize, f32, Vec<crate::common::config::EqBand>)>,
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
    },
    UpdateSpatialConfig {
        channel_positions: Vec<Option<crate::common::config::Point3D>>,
        room_zones: Vec<crate::common::config::RoomZone>,
        trajectory: Option<crate::common::config::Trajectory>,
    },
    UpdateTrajectoryPosition {
        position: crate::common::config::Point3D,
    },
    ApplyGlobalTuning {
        master_headroom_db: f32,
        peak_limiter_enabled: bool,
    },
}
