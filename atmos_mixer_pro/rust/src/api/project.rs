//! 프로젝트 파일을 다른 PC에서 열 때 트랙 오디오·도면 경로를 다시 연결한다(core::media_relink).
//! 다시 연결한 경로와 못 찾은 파일은 앱 로그에 남긴다.
use crate::common::config::AppConfig;
use crate::core::media_relink::{relink_tracks, MediaFinder};
use crate::core::state::GLOBAL_STATE;
use std::path::PathBuf;

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
