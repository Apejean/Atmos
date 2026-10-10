//! Load Project 전 프로젝트 파일 검사(api::project::api_check_project_file). 예전에는 읽을 수 없는 파일을 열면
//! api_get_config가 빈 기본 설정을 돌려주고 엔진 설정·채널 튜닝까지 바꿔, 지금 설정이 통째로 비워졌다
//! (2026-10-10 Windows). 검사는 해석만 하고 아무것도 바꾸지 않는다.
use rust_lib_atmos_mixer_pro::api::project::api_check_project_file;
use rust_lib_atmos_mixer_pro::common::config::AppConfig;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::path::{Path, PathBuf};

fn fresh_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("atmos_project_check_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, name: &str, content: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    path.to_string_lossy().into_owned()
}

#[test]
fn 앱이_저장한_프로젝트는_통과한다() {
    let dir = fresh_dir("ok");
    // 앱이 저장하는 모양: 엔진 설정 + 설계 데이터 항목(exhibition_design)
    let mut value = serde_json::to_value(AppConfig::default()).unwrap();
    value["exhibition_design"] = serde_json::json!({ "tuning_state": "{}" });
    let path = write(&dir, "show.atmos", &serde_json::to_string_pretty(&value).unwrap());
    assert!(api_check_project_file(path).is_ok());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 읽을_수_없는_파일은_이유와_함께_거절하고_아무것도_바꾸지_않는다() {
    let dir = fresh_dir("bad");
    let cases = [
        ("text.atmos", "이건 프로젝트가 아니다"),
        ("empty.atmos", ""),
        ("other.atmos", r#"{"name": "다른 프로그램 설정"}"#), // osc_port·buffer_size가 없다
        ("broken_room.atmos", r#"{"osc_port": 8000, "buffer_size": 512, "rooms": [{"id": "r1"}]}"#), // 방 필수 항목이 없다
    ];
    for (name, content) in cases {
        let path = write(&dir, name, content);
        let err = api_check_project_file(path).expect_err(name);
        assert!(err.message.contains("Atmos 프로젝트 파일이 아닙니다"), "{name}: {}", err.message);
    }

    let missing = dir.join("none.atmos").to_string_lossy().into_owned();
    let err = api_check_project_file(missing.clone()).expect_err("없는 파일");
    assert!(err.message.contains("읽을 수 없습니다"), "{}", err.message);
    assert!(!Path::new(&missing).exists(), "없는 파일을 기본 설정으로 만들었다");

    // 깨진 파일 옆에 사본(.corrupted.json)을 남기지 않고, 엔진 설정도 그대로다.
    let mut names: Vec<String> =
        std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    assert_eq!(names, ["broken_room.atmos", "empty.atmos", "other.atmos", "text.atmos"]);
    assert!(GLOBAL_STATE.config.read().unwrap().is_none(), "검사가 엔진 설정을 바꿨다");
    let _ = std::fs::remove_dir_all(dir);
}
