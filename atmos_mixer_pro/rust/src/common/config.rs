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
    /// 브로드밴드 흡음계수(0.0~1.0). 초기반사음 게인 계산(reflect_r = sqrt(1 - absorption_coeff))에 사용.
    #[serde(default)]
    pub absorption_coeff: f32,
    /// 수음점(리스너) 높이(m). 초기반사음 image-source 계산의 리스너 Z좌표로 사용. 기본 1.2m(프로젝트 표준).
    #[serde(default = "default_ear_level")]
    pub ear_level: f32,
}

fn default_ear_level() -> f32 {
    1.2
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Trajectory {
    pub waypoints: Vec<Point3D>,
    pub current_position: Point3D,
    #[serde(default)]
    pub target_room_zone_id: Option<String>,
}

// Copy를 붙인 이유: 오디오 스레드(dsp.rs의 smooth_eq_block)에서 밴드 타입을
// 꺼내 쓸 때 clone()을 쓰지 않기 위해서다(DSP Law 1은 오디오 콜백 안의
// .clone() 사용을 금지한다). 필드 없는 enum이라 Copy가 안전하다.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
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
    /// 로우컷/하이컷의 기울기(dB/oct): 12, 18, 24. 벨·쉘프·노치에서는 무시한다.
    /// 예전 config.json에는 이 필드가 없으므로 #[serde(default)]로 12가 들어간다.
    pub slope_db_per_oct: u32,
}

impl Default for EqBand {
    fn default() -> Self {
        Self {
            enabled: false,
            freq: 1000.0,
            gain: 0.0,
            q_factor: 0.707,
            filter_type: EqType::Bell,
            slope_db_per_oct: 12,
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
    #[serde(default)]
    pub phase_invert: bool,
    #[serde(default)]
    pub gain_db: f32,
}

#[flutter_rust_bridge::frb]
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
    pub global_reverb_mix: f32,
    #[serde(default)]
    pub global_reverb_decay: f32,
    /// OSC 응답·시작 신호를 보낼 곳("IP:포트" 목록, 규격 §3.1). 비면 보내지 않는다. 잘못된 항목은 보낼 때만
    /// 건너뛰고 로그에 남기며, 저장할 때는 사용자가 적은 문자열을 그대로 둔다(osc::reply).
    #[serde(default)]
    #[frb(default = "const []")]
    pub osc_reply_targets: Vec<String>,
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
            osc_reply_targets: Vec::new(),
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
        write_file_atomically(path.as_ref(), &content)?;
        Ok(())
    }

    /// 앱 설정(config.json) 저장. 지금 파일이 정상 JSON이면 먼저 `config.json.bak`으로 남겨(직전 정상본 하나)
    /// 다음 실행에서 손상을 발견해도 되살릴 수 있게 한다. 프로젝트 파일(.atmos)은 [`Self::save_to_file`]을 쓴다
    /// (프로젝트 폴더에 .bak을 만들지 않는다).
    pub fn save_with_backup<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let path = path.as_ref();
        if let Ok(current) = fs::read_to_string(path) {
            if serde_json::from_str::<AppConfig>(&current).is_ok() {
                // 백업 실패는 저장을 막지 않는다(로그만).
                if let Err(e) = write_file_atomically(&config_backup_path(path), &current) {
                    crate::core::state::GLOBAL_STATE
                        .log(format!("설정 백업을 쓰지 못했다(저장은 계속): {e}"));
                }
            }
        }
        self.save_to_file(path)
    }

    /// 앱 설정을 읽고, 없거나 손상됐으면 되살린다(설정 파일 손상 복구).
    /// 순서: 정상이면 그대로 → 저장 중 남은 임시 파일(`config.tmp`, 완전할 때)과 직전 정상본(`config.json.bak`) 중
    /// 더 최근 것 → 둘 다 없으면 기본값. 손상 원본은 `config.corrupted-<시각>.json`으로 보존하고 덮어쓰지 않는다.
    /// 되살린 내용은 config.json으로 다시 쓴다. 파일이 아예 없고 살릴 것도 없으면 첫 실행으로 보고 알리지 않는다.
    pub fn load_with_recovery<P: AsRef<Path>>(path: P) -> (Self, Option<ConfigRecovery>) {
        let path = path.as_ref();
        let reason = match read_with_retry(path) {
            Ok(content) => match serde_json::from_str::<AppConfig>(&content) {
                Ok(config) => {
                    // 정상이면 예전 충돌·저장 실패로 남은 임시 파일은 쓰지 않고 지운다.
                    let leftover = path.with_extension("tmp");
                    if leftover.exists() && fs::remove_file(&leftover).is_ok() {
                        crate::core::state::GLOBAL_STATE.log(format!(
                            "설정 파일은 정상이라 남아 있던 임시 파일을 지웠다: {}",
                            leftover.display()
                        ));
                    }
                    return (config, None);
                }
                Err(e) => {
                    let kept = preserve_corrupted(path);
                    RecoveryReason::Corrupted { error: e.to_string(), kept }
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => RecoveryReason::Missing,
            // 권한·공유 위반·백신이 잠깐 잡고 있음 등: 파일은 멀쩡할 수 있으니 손상으로 보지 않는다(덮지 않는다).
            Err(e) => RecoveryReason::Unreadable { error: e.to_string() },
        };
        // config.json을 되살린 내용으로 다시 써도 되는가. 못 읽은 경우와 손상본을 보존하지 못한 경우에는
        // 원본(최근 변경일 수 있음)을 지키려고 메모리에서만 쓰고, 다음 정상 저장 때 바뀌게 둔다.
        let may_overwrite = reason.may_overwrite();

        // 읽을 수 있는 후보 중 가장 최근에 쓴 것을 쓴다(예전 충돌에서 남은 오래된 임시 파일이 더 새 백업을
        // 이기지 않게). 수정 시각이 같거나 모르면 임시 파일이 먼저다(저장 직전 내용).
        let mut candidates: Vec<(std::time::SystemTime, RecoverySource, AppConfig)> = Vec::new();
        for (source, candidate) in [
            (RecoverySource::Temp, path.with_extension("tmp")),
            (RecoverySource::Backup, config_backup_path(path)),
        ] {
            let Ok(content) = fs::read_to_string(&candidate) else { continue };
            let Ok(config) = serde_json::from_str::<AppConfig>(&content) else { continue };
            let modified = fs::metadata(&candidate)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            candidates.push((modified, source, config));
        }
        if let Some(best) = candidates.iter().map(|c| c.0).max() {
            if let Some(idx) = candidates.iter().position(|c| c.0 == best) {
                let (_, source, config) = candidates.swap_remove(idx);
                if may_overwrite {
                    let _ = config.save_to_file(path);
                }
                return (config, Some(ConfigRecovery { reason, source }));
            }
        }

        let config = Self::default();
        match reason {
            // 첫 실행: 기본 설정 파일을 만든다(지금까지와 같다).
            RecoveryReason::Missing => {
                let _ = config.save_to_file(path);
                (config, None)
            }
            // 손상됐거나 읽지 못했는데 살릴 것이 없다: 기본값으로 시작하고 원본은 덮지 않는다.
            reason => {
                (config, Some(ConfigRecovery { reason, source: RecoverySource::Default }))
            }
        }
    }
}

/// 설정을 되살린 까닭.
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryReason {
    /// config.json이 없었다.
    Missing,
    /// 읽었지만 해석하지 못했다(손상). `kept`는 보존한 손상본 경로(보존 실패면 None).
    Corrupted { error: String, kept: Option<String> },
    /// 다시 시도해도 읽지 못했다(파일은 멀쩡할 수 있다). config.json은 덮지 않는다.
    Unreadable { error: String },
}

impl RecoveryReason {
    /// 되살린 내용으로 config.json을 다시 써도 되는가.
    pub fn may_overwrite(&self) -> bool {
        match self {
            RecoveryReason::Missing => true,
            RecoveryReason::Corrupted { kept, .. } => kept.is_some(),
            RecoveryReason::Unreadable { .. } => false,
        }
    }
}

/// 설정을 어디서 되살렸는가.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoverySource {
    Temp,
    Backup,
    Default,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigRecovery {
    pub reason: RecoveryReason,
    pub source: RecoverySource,
}

impl ConfigRecovery {
    /// 사건 로그 한 줄(`사건: config_recovered …`)의 값들.
    pub fn log_fields(&self) -> (String, String) {
        let reason = match &self.reason {
            RecoveryReason::Missing => "missing".to_string(),
            RecoveryReason::Corrupted { .. } => "corrupted".to_string(),
            RecoveryReason::Unreadable { .. } => "unreadable".to_string(),
        };
        let source = match self.source {
            RecoverySource::Temp => "temp",
            RecoverySource::Backup => "backup",
            RecoverySource::Default => "default",
        };
        (reason, source.to_string())
    }

    /// 화면에 한 번 보일 안내.
    pub fn notice(&self) -> String {
        let what = match &self.reason {
            RecoveryReason::Missing => "설정 파일(config.json)이 없어졌습니다".to_string(),
            RecoveryReason::Corrupted { kept: Some(k), .. } => {
                format!("설정 파일(config.json)이 손상되어 읽지 못했습니다. 손상된 파일은 {k}에 보존했습니다")
            }
            RecoveryReason::Corrupted { kept: None, .. } => {
                "설정 파일(config.json)이 손상되어 읽지 못했고 손상본을 따로 보존하지 못해 원본은 그대로 두었습니다(다음 저장 때 바뀝니다)".to_string()
            }
            RecoveryReason::Unreadable { .. } => {
                "설정 파일(config.json)을 읽지 못했습니다(다른 프로그램이 잡고 있을 수 있음). 파일은 그대로 두었습니다(다음 저장 때 바뀝니다)".to_string()
            }
        };
        let how = match self.source {
            RecoverySource::Temp => "저장 중이던 마지막 설정으로 되살렸습니다.",
            RecoverySource::Backup => "직전에 정상이던 설정으로 되살렸습니다. 마지막 변경 하나가 빠졌을 수 있으니 확인해 주세요.",
            RecoverySource::Default => "되살릴 백업이 없어 기본 설정으로 시작했습니다. 프로젝트 파일(.atmos)을 다시 열어 주세요.",
        };
        format!("{what}. {how}")
    }
}

/// NotFound가 아닌 읽기 오류(권한·공유 위반·백신이 잠깐 잡고 있음)는 짧게 다시 시도한다(200ms 간격 3회).
fn read_with_retry(path: &Path) -> std::io::Result<String> {
    let mut last = fs::read_to_string(path);
    for _ in 0..3 {
        match &last {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                std::thread::sleep(std::time::Duration::from_millis(200));
                last = fs::read_to_string(path);
            }
            _ => break,
        }
    }
    last
}

/// 직전 정상본 경로(`config.json` → `config.json.bak`).
pub fn config_backup_path(path: &Path) -> std::path::PathBuf {
    let mut p = path.as_os_str().to_owned();
    p.push(".bak");
    std::path::PathBuf::from(p)
}

/// 손상본을 `<이름>.corrupted-<시각>.json`으로 복사한다(이전 손상본을 덮어쓰지 않는다).
fn preserve_corrupted(path: &Path) -> Option<String> {
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let stem = path.file_stem()?.to_string_lossy().into_owned();
    let prefix = format!("{stem}.corrupted-");
    // 같은 초에 이미 손상본이 있으면 그 번호들의 다음 번호를 쓴다. 정리로 지워진 이름을 다시 쓰면 새 손상본이
    // 가장 오래된 것으로 정렬되어 바로 지워졌다.
    let dir = match path.parent() {
        Some(d) if !d.as_os_str().is_empty() => d.to_path_buf(),
        _ => std::path::PathBuf::from("."),
    };
    let next = fs::read_dir(&dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.file_name().map_or(false, |n| n.to_string_lossy().starts_with(&prefix)))
                .map(|p| corrupted_copy_order(&p, &prefix))
                .filter(|(s, _)| *s == stamp)
                .map(|(_, n)| n + 1)
                .max()
        })
        .flatten();
    let target = match next {
        None => path.with_file_name(format!("{prefix}{stamp}.json")),
        Some(n) => path.with_file_name(format!("{prefix}{stamp}-{n}.json")),
    };
    fs::copy(path, &target).ok()?;
    prune_corrupted(path, MAX_CORRUPTED_COPIES);
    Some(target.to_string_lossy().into_owned())
}

/// 보존하는 손상본 최대 개수(손상이 되풀이돼도 쌓이지 않게).
pub const MAX_CORRUPTED_COPIES: usize = 5;

/// 손상본을 이름(시각) 순으로 보고 오래된 것부터 지워 `keep`개만 남긴다.
fn prune_corrupted(path: &Path, keep: usize) {
    let (Some(dir), Some(stem)) = (path.parent(), path.file_stem()) else { return };
    let prefix = format!("{}.corrupted-", stem.to_string_lossy());
    let dir = if dir.as_os_str().is_empty() { Path::new(".") } else { dir };
    let Ok(entries) = fs::read_dir(dir) else { return };
    let mut copies: Vec<std::path::PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .map_or(false, |n| n.to_string_lossy().starts_with(&prefix))
        })
        .collect();
    if copies.len() <= keep {
        return;
    }
    // 이름의 시각 도장(`<시각>[-n]`)으로 정렬한다. Windows의 fs::copy는 원본의 수정 시각을 가져오므로 수정
    // 시각은 보존한 순서와 다를 수 있다.
    copies.sort_by_key(|p| corrupted_copy_order(p, &prefix));
    for old in &copies[..copies.len() - keep] {
        let _ = fs::remove_file(old);
    }
}

/// 손상본 이름 `<prefix><YYYYMMDD-HHMMSS>[-n].json`의 순서 키(시각 도장, 같은 초 안의 번호).
fn corrupted_copy_order(p: &Path, prefix: &str) -> (String, u32) {
    let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let rest = name.strip_prefix(prefix).unwrap_or(&name);
    let rest = rest.strip_suffix(".json").unwrap_or(rest);
    match rest.get(15..).and_then(|tail| tail.strip_prefix('-')) {
        Some(n) => (rest[..15].to_string(), n.parse().unwrap_or(0)),
        None => (rest.to_string(), 0),
    }
}

/// 임시 파일(`<이름>.tmp`)에 쓰고 디스크에 내린 뒤 이름을 바꾼다. 원본을 먼저 지우지 않는다(지우고 바꾸는 사이에
/// 꺼지면 파일이 없어져 다음 실행이 기본값으로 덮었다). Windows의 rename도 기존 파일을 바꿔친다.
fn write_file_atomically(path: &Path, content: &str) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let tmp_path = if path.extension().map_or(false, |e| e == "bak") {
        let mut p = path.as_os_str().to_owned();
        p.push(".tmp");
        std::path::PathBuf::from(p)
    } else {
        path.with_extension("tmp")
    };
    {
        let mut file = fs::File::create(&tmp_path)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(&tmp_path, path)
}

#[cfg(test)]
mod tests {
    use super::RoomZone;

    /// B-1 완료 기준: 빈 JSON `{}`을 RoomZone으로 역직렬화했을 때
    /// absorption_coeff=0.0(암묵적 f32 기본값), ear_level=1.2(명시적 기본 함수)가 채워지는지 검증.
    #[test]
    fn room_zone_defaults_absorption_and_ear_level() {
        let zone: RoomZone = serde_json::from_str("{}").expect("empty RoomZone JSON must parse");
        assert_eq!(zone.absorption_coeff, 0.0);
        assert_eq!(zone.ear_level, 1.2);
    }
}
