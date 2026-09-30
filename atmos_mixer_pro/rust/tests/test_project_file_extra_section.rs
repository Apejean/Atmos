//! 프로젝트 파일(.atmos)에는 엔진이 모르는 "설계 데이터" 항목이 함께 들어간다
//! (Flutter `core/state/project_file.dart`의 `exhibition_design`). 스피커 배치·방·
//! 리버브처럼 엔진 설정에 없는 값들을 파일 하나로 현장에 옮기기 위한 것이다.
//!
//! 엔진은 이 항목을 무시하고 자기 설정만 읽어야 한다. 이게 깨지면(예: serde에
//! deny_unknown_fields가 붙으면) 파싱이 실패하고, 현장 컴퓨터에서 프로젝트를 열 때
//! 장치·버퍼·라우팅이 전부 기본값으로 떨어진다.

use rust_lib_atmos_mixer_pro::api::simple::api_get_config;

#[test]
fn 설계_데이터_항목이_있어도_엔진_설정을_그대로_읽는다() {
    let dir = std::env::temp_dir().join(format!("atmos_project_file_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("임시 디렉터리 생성 실패");
    let path = dir.join("project.atmos");
    let json = r#"{
        "osc_port": 9123,
        "device_name": "[ASIO] RME Fireface",
        "buffer_size": 1024,
        "exhibition_design": {
            "exhibition_speaker_layout": "[{\"id\":\"spk_0\",\"channel\":0}]",
            "blueprint_scale": 60.0
        }
    }"#;
    std::fs::write(&path, json).expect("파일 쓰기 실패");

    let config = api_get_config(path.to_string_lossy().to_string());

    assert_eq!(
        config.osc_port, 9123,
        "설계 데이터 항목 때문에 파싱이 실패해 기본값으로 떨어졌다"
    );
    assert_eq!(config.device_name.as_deref(), Some("[ASIO] RME Fireface"));
    assert_eq!(config.buffer_size, 1024);

    let _ = std::fs::remove_dir_all(&dir);
}
