//! 테스트 기록이 실제 앱 로그에 섞이지 않는다(HANDOFF 남은 일 14). `cargo test`·`cargo run`은
//! rust/.cargo/config.toml의 [env]로 ATMOS_LOG_DIR(rust/target/test-logs)을 받아 그곳에 쓴다.
use rust_lib_atmos_mixer_pro::core::log_file::{log_dir, LOG_FILE_NAME};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::path::{Path, PathBuf};

/// 앱이 실제로 쓰는 로그 폴더(Windows %TEMP%, macOS TMPDIR 아래).
fn real_app_log_dir() -> PathBuf {
    std::env::temp_dir().join("atmos_mixer_pro_logs")
}

#[test]
fn uses_the_test_log_folder_under_cargo() {
    let dir = log_dir();
    assert_ne!(dir, real_app_log_dir(), "테스트가 실제 앱 로그 폴더에 쓴다: {}", dir.display());
    assert!(dir.ends_with(Path::new("target").join("test-logs")), "{}", dir.display());
}

#[test]
fn app_log_lines_land_in_the_test_folder() {
    let marker = format!("log-isolation-marker-{}", std::process::id());
    GLOBAL_STATE.log(marker.clone());
    let written = std::fs::read_to_string(log_dir().join(LOG_FILE_NAME)).unwrap_or_default();
    assert!(written.contains(&marker), "테스트 폴더 로그에 표시가 없다");
    let real = std::fs::read_to_string(real_app_log_dir().join(LOG_FILE_NAME)).unwrap_or_default();
    assert!(!real.contains(&marker), "실제 앱 로그에 테스트 줄이 들어갔다");
}
