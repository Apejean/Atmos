//! 설계 데이터(SharedPreferences, `shared_preferences.json`) 보호(검토 보고서 A3, 안 B).
//!
//! Windows의 shared_preferences 플러그인은 값 하나를 바꿀 때마다 전체 맵을 같은 파일에 덮어쓴다(임시 파일·sync 없음).
//! 쓰는 도중 꺼지면 0바이트나 잘린 JSON이 남고, 0바이트면 설계 데이터(스피커 배치·튜닝 등)를 조용히 잃고,
//! 잘렸으면 플러그인의 `json.decode`가 예외를 던져 설정을 읽는 화면들이 실패한다.
//! 플러그인의 쓰기 경로는 그대로 두고, 앱이 처음 읽기 전에 검사·복구하고([`check_prefs_file`]),
//! 실행 중에는 정상본을 갱신한다([`backup_if_changed`], [`start_periodic_backup`]).
//! 코드와 규칙은 config.json 복구(`common::config`)와 같다(원자적 쓰기, 손상본 최근 5개 보존).

use crate::common::config::{config_backup_path, preserve_corrupted, read_with_retry, write_file_atomically};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// 시작 전 검사 결과.
#[derive(Debug, Clone, PartialEq)]
pub enum PrefsOutcome {
    /// 파일이 없다(첫 실행, macOS). 아무것도 하지 않는다.
    Missing,
    /// 정상. 정상본(`.bak`)을 갱신했다.
    Healthy,
    /// 다시 시도해도 읽지 못했다(잠김 등). 아무것도 바꾸지 않는다.
    Unreadable { error: String },
    /// 비었거나 깨져 정상본으로 되살렸다.
    Recovered { kept: Option<String> },
    /// 비었거나 깨졌는데 정상본이 없어 빈 객체로 바꿨다(손상본은 보존).
    Reset { kept: Option<String> },
    /// 깨졌는데 손상본을 보존하지 못해 원본을 그대로 두었다.
    LeftBroken { error: String },
}

impl PrefsOutcome {
    /// 화면에 한 번 보일 안내(되살렸거나 비웠을 때만).
    pub fn notice(&self) -> Option<String> {
        match self {
            PrefsOutcome::Recovered { .. } => Some(
                "설계 데이터 파일(shared_preferences.json)이 손상되어 직전 정상본으로 되살렸습니다. \
                 마지막 1분 안팎의 변경(스피커 배치·튜닝 등)이 빠졌을 수 있으니 확인해 주세요."
                    .to_string(),
            ),
            PrefsOutcome::Reset { .. } => Some(
                "설계 데이터 파일(shared_preferences.json)이 손상되었고 되살릴 정상본이 없어 빈 설정으로 시작했습니다. \
                 마지막으로 저장한 프로젝트 파일(.atmos)을 다시 열면 스피커 배치·튜닝을 되살릴 수 있습니다."
                    .to_string(),
            ),
            PrefsOutcome::LeftBroken { .. } => Some(
                "설계 데이터 파일(shared_preferences.json)이 손상되었는데 손상본을 따로 보존하지 못해 그대로 두었습니다. \
                 설정 화면이 열리지 않으면 마지막 프로젝트 파일(.atmos)을 다시 열어 주세요."
                    .to_string(),
            ),
            _ => None,
        }
    }
}

/// 플러그인이 읽을 수 있는 정상 내용인가(JSON 객체). 0바이트·공백·배열·잘린 JSON은 아니다.
fn is_valid_prefs(content: &str) -> bool {
    matches!(
        serde_json::from_str::<serde_json::Value>(content),
        Ok(serde_json::Value::Object(_))
    )
}

fn backup_path(path: &Path) -> PathBuf {
    config_backup_path(path)
}

fn log_event(kind: &str, path: &Path, kept: &Option<String>) {
    crate::core::event_log::log_event(
        kind,
        &[
            ("path", &path.to_string_lossy()),
            ("kept", kept.as_deref().unwrap_or("none")),
        ],
    );
}

/// 앱이 SharedPreferences를 처음 읽기 전에 부른다. 파일이 없으면 아무것도 하지 않는다.
pub fn check_prefs_file(path: &Path) -> PrefsOutcome {
    let content = match read_with_retry(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return PrefsOutcome::Missing,
        Err(e) => {
            crate::core::state::GLOBAL_STATE.log(format!(
                "설계 데이터 파일을 읽지 못해 그대로 둔다(다른 프로그램이 잡고 있을 수 있음): {e}"
            ));
            return PrefsOutcome::Unreadable { error: e.to_string() };
        }
    };
    if is_valid_prefs(&content) {
        if let Err(e) = write_file_atomically(&backup_path(path), &content) {
            crate::core::state::GLOBAL_STATE.log(format!("설계 데이터 정상본을 쓰지 못했다: {e}"));
        }
        return PrefsOutcome::Healthy;
    }

    // 비었거나 깨졌다. 손상본을 먼저 보존한다(0바이트도 증거로 남긴다).
    let kept = preserve_corrupted(path);
    let backup = fs::read_to_string(backup_path(path))
        .ok()
        .filter(|b| is_valid_prefs(b));
    if kept.is_none() && !content.trim().is_empty() {
        // 내용이 있는 손상본을 보존하지 못했으면 원본을 덮지 않는다(되살릴 단서를 지운다).
        let out = PrefsOutcome::LeftBroken { error: "손상본 보존 실패".to_string() };
        log_event("prefs_left_broken", path, &kept);
        return out;
    }
    match backup {
        Some(b) => match write_file_atomically(path, &b) {
            Ok(()) => {
                log_event("prefs_recovered", path, &kept);
                PrefsOutcome::Recovered { kept }
            }
            Err(e) => PrefsOutcome::LeftBroken { error: e.to_string() },
        },
        None => match write_file_atomically(path, "{}") {
            Ok(()) => {
                log_event("prefs_reset", path, &kept);
                PrefsOutcome::Reset { kept }
            }
            Err(e) => PrefsOutcome::LeftBroken { error: e.to_string() },
        },
    }
}

/// 파일이 마지막으로 바뀐 지 `min_age` 넘게 지났고 정상이며 정상본과 다르면 정상본을 갱신한다(갱신했으면 true).
/// 쓰기 중간의 파일은 해석에 실패하거나 너무 최근이라 건너뛴다. 읽기는 짧게 열고 바로 닫으며, std는 읽기·쓰기·삭제를
/// 공유하는 모드로 열어 플러그인의 쓰기를 막지 않는다.
pub fn backup_if_changed(path: &Path, min_age: Duration) -> bool {
    let Ok(meta) = fs::metadata(path) else { return false };
    let age = meta
        .modified()
        .ok()
        .and_then(|m| SystemTime::now().duration_since(m).ok())
        .unwrap_or(Duration::ZERO);
    if age < min_age {
        return false;
    }
    let Ok(content) = fs::read_to_string(path) else { return false };
    if !is_valid_prefs(&content) {
        return false;
    }
    let bak = backup_path(path);
    if fs::read_to_string(&bak).ok().as_deref() == Some(content.as_str()) {
        return false;
    }
    write_file_atomically(&bak, &content).is_ok()
}

/// 실행 중 60초마다 정상본을 갱신하는 스레드를 띄운다(마지막 변경 뒤 2초가 지나야 읽는다). 오디오 스레드와 무관.
pub fn start_periodic_backup(path: PathBuf) {
    let spawned = std::thread::Builder::new()
        .name("prefs-backup".to_string())
        .spawn(move || loop {
            std::thread::sleep(Duration::from_secs(60));
            let _ = backup_if_changed(&path, Duration::from_secs(2));
        });
    if spawned.is_err() {
        crate::core::state::GLOBAL_STATE.log("설계 데이터 정상본 갱신 스레드를 만들지 못했다".to_string());
    }
}
