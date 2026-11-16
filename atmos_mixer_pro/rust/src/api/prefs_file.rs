//! 설계 데이터 파일(shared_preferences.json) 보호(검토 보고서 A3, core::prefs_guard).

/// 앱이 SharedPreferences를 처음 읽기 전에(main.dart, runApp 전) 부른다. 비었거나 깨졌으면 정상본으로 되살리거나
/// 빈 객체로 바꾸고, 화면에 한 번 보일 안내를 돌려준다. 파일이 없으면(첫 실행, macOS) 아무것도 하지 않는다.
/// 파일이 있으면(또는 Windows에서) 60초마다 정상본을 갱신하는 스레드를 띄운다.
pub fn api_check_prefs_file(path: String) -> Option<String> {
    let path = std::path::PathBuf::from(path);
    let outcome = crate::core::prefs_guard::check_prefs_file(&path);
    let notice = outcome.notice();
    if outcome != crate::core::prefs_guard::PrefsOutcome::Missing || cfg!(windows) {
        crate::core::prefs_guard::start_periodic_backup(path);
    }
    notice
}
