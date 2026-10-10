//! File > Export Project는 프로젝트 파일만 쓰고 엔진의 지금 설정은 그대로 둔다(api_write_project_file).
//!
//! 예전에는 내보내기가 `api_save_config`로 project.atmos를 썼다. 이 함수는 파일을 쓰면서 엔진의 지금 설정
//! (`GLOBAL_STATE.config`)까지 넘겨받은 설정으로 바꾼다. 내보낼 설정은 트랙 경로가 복사본을 가리키므로, 내보낸 순간
//! 지금 프로젝트가 복사본을 가리키게 됐고 곧 앱의 config.json에도 그 경로가 저장됐다(2026-10-10 Mac 확인).
//! 내보낸 폴더를 지우거나 USB를 빼면 원래 PC의 프로젝트가 음원을 잃는다.
use rust_lib_atmos_mixer_pro::api::project::{api_check_project_file, api_write_project_file};
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;

/// AppConfig에는 PartialEq가 없어 JSON 값으로 견준다.
fn json(config: &AppConfig) -> serde_json::Value {
    serde_json::to_value(config).unwrap()
}

fn config_with_track(file_path: &str) -> AppConfig {
    AppConfig {
        rooms: vec![RoomConfig {
            id: "r1".into(),
            name: "한글 방".into(),
            color_hex: "#ffffff".into(),
            volume: 1.0,
            volume_osc_address: String::new(),
            clear_osc_address: String::new(),
            tracks: vec![TrackConfig {
                id: "bgm".into(),
                name: "배경음악".into(),
                file_path: file_path.into(),
                volume: 1.0,
                is_loop: true,
                is_streaming: false,
                output_channel: 1,
                output_stereo: false,
                play_osc_address: String::new(),
                stop_osc_address: String::new(),
            }],
        }],
        ..AppConfig::default()
    }
}

#[test]
fn 내보낼_프로젝트_파일을_써도_엔진의_지금_설정은_그대로다() {
    let dir = std::env::temp_dir().join(format!("atmos_write_project_{}", std::process::id()));
    let path = dir.join("Atmos_Project").join("project.atmos");
    let working = config_with_track("/원래/음원/배경음악.wav");
    let exported = config_with_track("/내보낸/폴더/audio/배경음악.wav");
    *GLOBAL_STATE.config.write().unwrap() = Some(working.clone());
    let version = GLOBAL_STATE.config_version.load(Ordering::SeqCst);

    api_write_project_file(path.to_string_lossy().into_owned(), exported.clone()).unwrap();

    // 파일은 내보낸 설정(복사본 경로)이고, Load Project 검사도 통과한다. 폴더가 없으면 만든다.
    assert_eq!(json(&AppConfig::load_from_file(&path).unwrap()), json(&exported));
    api_check_project_file(path.to_string_lossy().into_owned()).unwrap();
    // 엔진의 지금 설정은 그대로다
    let now = GLOBAL_STATE.config.read().unwrap().clone().expect("엔진 설정이 비었다");
    assert_eq!(json(&now), json(&working), "내보내기가 엔진의 지금 설정을 바꿨다");
    assert_eq!(GLOBAL_STATE.config_version.load(Ordering::SeqCst), version, "설정 판이 바뀌었다");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn 쓸_수_없는_곳이면_이유를_돌려준다() {
    // 일반 파일 아래에 폴더를 만들 수 없다.
    let blocker = std::env::temp_dir().join(format!("atmos_write_project_blocker_{}", std::process::id()));
    std::fs::write(&blocker, b"not a folder").unwrap();
    let path = blocker.join("project.atmos");
    let err = api_write_project_file(path.to_string_lossy().into_owned(), AppConfig::default());
    assert!(err.is_err(), "쓸 수 없는 경로인데 성공했다");
    let _ = std::fs::remove_file(&blocker);
}
