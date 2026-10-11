//! 프로젝트 파일을 다른 PC에서 열 때 트랙 오디오·도면 경로를 다시 연결한다(core::media_relink).
//! 다시 연결한 경로와 못 찾은 파일은 앱 로그에 남긴다.
use crate::api::error::AtmosError;
use crate::common::config::AppConfig;
use crate::core::media_relink::{relink_tracks, MediaFinder};
use crate::core::state::GLOBAL_STATE;
use std::path::PathBuf;

/// Load Project 전에 부른다. 프로젝트 파일을 앱 설정과 같은 규칙으로 해석해 보기만 하고 상태는 바꾸지 않는다.
/// 읽을 수 없거나 Atmos 프로젝트가 아니면 이유를 돌려준다. `api_get_config`는 앱 자신의 config.json용이라 읽기에
/// 실패하면 기본 설정을 돌려주고 엔진 설정·채널 튜닝까지 바꿔, 잘못 고른 파일로 지금 설정이 비워졌다
/// (2026-10-10 Windows). 없는 파일을 기본 설정으로 만들거나 깨진 파일의 사본을 남기지도 않는다.
pub fn api_check_project_file(path: String) -> Result<(), AtmosError> {
    let checked = std::fs::read_to_string(&path)
        .map_err(|e| format!("프로젝트 파일을 읽을 수 없습니다: {e}"))
        .and_then(|content| {
            serde_json::from_str::<AppConfig>(&content)
                .map(|_| ())
                .map_err(|e| format!("Atmos 프로젝트 파일이 아닙니다(설정을 해석하지 못했습니다: {e})"))
        });
    checked.map_err(|message| {
        GLOBAL_STATE.log(format!("프로젝트 열기 거절: {path} — {message}"));
        AtmosError { message }
    })
}

/// File > Export Project가 부른다. 프로젝트 파일만 쓰고 엔진의 지금 설정·채널 상태는 바꾸지 않는다.
/// `api_save_config`는 파일을 쓰면서 엔진 설정까지 넘겨받은 설정으로 바꾼다. 내보낼 설정은 트랙·도면 경로가
/// 복사본을 가리키므로, 그것으로 쓰면 지금 프로젝트까지 복사본을 가리키게 됐다(2026-10-10 Mac 확인). 폴더가 없으면
/// 만들고, 쓰기는 앱 설정과 같은 원자적 쓰기다(`AppConfig::save_to_file`).
pub fn api_write_project_file(path: String, config: AppConfig) -> Result<(), AtmosError> {
    config.save_to_file(&path).map_err(|e| AtmosError {
        message: format!("프로젝트 파일을 쓸 수 없습니다: {e}"),
    })
}

/// 다시 연결한 엔진 설정과 끝까지 못 찾은 파일.
pub struct RelinkedConfig {
    pub config: AppConfig,
    /// 못 찾은 파일의 원래 경로(같은 경로는 한 번만).
    pub missing: Vec<String>,
}

/// 트랙 오디오 경로를 [search_dirs](앞의 것 먼저)와 그 하위 폴더에서 파일 이름으로 다시 연결한다.
/// 저장된 경로에 파일이 있으면 그대로 둔다. 엔진 설정의 다른 값은 건드리지 않는다.
pub fn api_relink_track_paths(config: AppConfig, search_dirs: Vec<String>) -> RelinkedConfig {
    let mut config = config;
    let dirs: Vec<PathBuf> = search_dirs.into_iter().map(PathBuf::from).collect();
    let report = relink_tracks(&mut config, &dirs);
    for (from, to) in &report.relinked {
        GLOBAL_STATE.log(format!("프로젝트 파일 다시 연결: {from} → {to}"));
    }
    for path in &report.missing {
        GLOBAL_STATE.log(format!("프로젝트 파일을 찾지 못했다: {path}"));
    }
    RelinkedConfig { config, missing: report.missing }
}

/// 파일 하나(도면 이미지)를 같은 방법으로 찾는다. 저장된 경로에 있으면 그 경로, 못 찾으면 None.
pub fn api_find_media(path: String, search_dirs: Vec<String>) -> Option<String> {
    let dirs: Vec<PathBuf> = search_dirs.into_iter().map(PathBuf::from).collect();
    let found = MediaFinder::new(&dirs).resolve(&path).map(|p| p.to_string_lossy().into_owned());
    match &found {
        Some(to) if to != &path => GLOBAL_STATE.log(format!("프로젝트 파일 다시 연결: {path} → {to}")),
        None => GLOBAL_STATE.log(format!("프로젝트 파일을 찾지 못했다: {path}")),
        _ => {}
    }
    found
}
