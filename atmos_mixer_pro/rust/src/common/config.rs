use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Point3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    #[serde(default)]
    pub yaw_rotation: f32,
    #[serde(default)]
    pub pitch_tilt: f32,
    #[serde(default)]
    pub dispersion_angle: f32,
    #[serde(default)]
    pub size: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct RoomZone {
    pub room_id: u32,
    pub boundary_min: Point3D,
    pub boundary_max: Point3D,
    #[serde(default)]
    pub boundary_delay_ms: f32,
    #[serde(default)]
    pub boundary_eq_bands: Vec<EqBand>,
    #[serde(default)]
    pub transmission_loss_db: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Trajectory {
    pub waypoints: Vec<Point3D>,
    pub current_position: Point3D,
    #[serde(default)]
    pub target_room_zone_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum EqType {
    #[default]
    LowCut,
    LowShelf,
    Bell,
    Notch,
    HighShelf,
    HighCut,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct EqBand {
    pub enabled: bool,
    pub freq: f32,
    pub gain: f32,
    pub q_factor: f32,
    pub filter_type: EqType,
}

impl Default for EqBand {
    fn default() -> Self {
        Self {
            enabled: false,
            freq: 1000.0,
            gain: 0.0,
            q_factor: 0.707,
            filter_type: EqType::Bell,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelSetting {
    pub enabled: bool,
    pub custom_name: String,
    #[serde(default)]
    pub delay_ms: f32,
    #[serde(default)]
    pub eq_bands: Vec<EqBand>,
    #[serde(default)]
    pub position: Option<Point3D>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub osc_port: u16,
    pub device_name: Option<String>,
    pub buffer_size: u32,
    #[serde(default)]
    pub theme_start_osc_address: String,
    #[serde(default)]
    pub system_reset_osc_address: String,
    #[serde(default)]
    pub tracking_osc_address: Option<String>,
    #[serde(default)]
    pub mono_configs: HashMap<u32, ChannelSetting>,
    #[serde(default)]
    pub stereo_configs: HashMap<u32, ChannelSetting>,
    #[serde(default)]
    pub multi_configs: HashMap<u32, ChannelSetting>,
    #[serde(default)]
    pub rooms: Vec<RoomConfig>,
    #[serde(default)]
    pub global_trajectory: Option<Trajectory>,
    #[serde(default)]
    pub room_zones: Vec<RoomZone>,
    #[serde(default)]
    pub is_exhibition_mode: bool,
    #[serde(default = "default_master_headroom")]
    pub master_headroom_db: f32,
    #[serde(default = "default_true")]
    pub peak_limiter_enabled: bool,
    #[serde(default)]
    pub osc_whitelist: Vec<String>,
    #[serde(default)]
    pub global_reverb_mix: f32,
    #[serde(default)]
    pub global_reverb_decay: f32,
}

fn default_master_headroom() -> f32 {
    0.0
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            osc_port: 8000,
            device_name: None,
            buffer_size: 256,
            theme_start_osc_address: String::new(),
            system_reset_osc_address: String::new(),
            tracking_osc_address: Some("/tracker/pos".to_string()),
            mono_configs: HashMap::new(),
            stereo_configs: HashMap::new(),
            multi_configs: HashMap::new(),
            rooms: Vec::new(),
            global_trajectory: None,
            room_zones: Vec::new(),
            global_reverb_mix: 0.0,
            global_reverb_decay: 1.0,
            is_exhibition_mode: false,
            master_headroom_db: 0.0,
            peak_limiter_enabled: true,
            osc_whitelist: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomConfig {
    pub id: String,
    pub name: String,
    pub color_hex: String,
    pub volume: f32, // 0.0 to 1.0
    #[serde(default)]
    pub volume_osc_address: String,
    #[serde(default)]
    pub clear_osc_address: String,
    #[serde(default)]
    pub tracks: Vec<TrackConfig>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackConfig {
    pub id: String,
    pub name: String,
    pub file_path: String,
    pub volume: f32,         // 0.0 to 1.0
    pub is_loop: bool,       // true = BGM, false = SFX
    #[serde(default)]
    pub is_streaming: bool,  // true = disk streaming, false = memory preload
    pub output_channel: u32, // 1 to 24 (1-indexed for user, mapped to 0-23 internally)
    #[serde(default = "default_true")]
    pub output_stereo: bool,
    #[serde(default)]
    pub play_osc_address: String,
    #[serde(default)]
    pub stop_osc_address: String,
}

impl AppConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            // 파일이 없으면 기본값으로 생성
            let default_config = Self::default();
            default_config.save_to_file(path)?;
            return Ok(default_config);
        }
        let content = fs::read_to_string(path)?;
        match serde_json::from_str(&content) {
            Ok(config) => Ok(config),
            Err(e) => {
                let backup_path = path.with_extension("corrupted.json");
                let _ = fs::copy(path, backup_path);
                Err(e.into())
            }
        }
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        // 부모 디렉토리가 없으면 생성
        let path_ref = path.as_ref();
        if let Some(parent) = path_ref.parent() {
            fs::create_dir_all(parent)?;
        }

        // 원자적 쓰기(Atomic Write) 적용: tmp에 먼저 쓰고 rename (OS 수준 안전 보장)
        let tmp_path = path_ref.with_extension("tmp");
        fs::write(&tmp_path, content)?;
        if path_ref.exists() {
            let _ = fs::remove_file(path_ref);
        }
        fs::rename(&tmp_path, path_ref)?;

        Ok(())
    }
}
