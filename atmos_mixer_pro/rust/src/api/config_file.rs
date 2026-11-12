//! 앱 설정 파일 상태를 화면에 알린다(설정 파일 손상 복구, common::config::AppConfig::load_with_recovery).

/// 시작할 때 설정 파일을 되살렸으면 그 안내를 한 번 돌려준다(없으면 None). 앱 시작 절차가 끝난 뒤 부른다.
pub fn api_take_config_notice() -> Option<String> {
    crate::core::config_notice::take()
}
