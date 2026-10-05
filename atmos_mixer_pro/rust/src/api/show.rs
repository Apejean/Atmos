//! 공연 시작·이어 가기(docs/superpowers/specs/2026-10-05-crash-relaunch-supervisor-design.md 4.4).
use crate::api::error::AtmosError;
use crate::api::simple::{api_play_track, api_set_active_room, api_stop_all};
use crate::core::state::GLOBAL_STATE;

/// 지난 공연 상태를 읽어 두고 5초마다 저장을 시작한다. 앱 시작 때(시작 관문 다음) 한 번 부른다.
/// [dir]는 앱 지원 폴더(설정 파일과 같은 곳)다.
pub fn api_start_show_state(dir: String) {
    crate::core::show_state::start(std::path::PathBuf::from(dir), crate::core::show_state::SAVE_INTERVAL);
}

/// 감시가 다시 띄웠거나(--auto-relaunched) 로그인 자동 실행이면, 앱 시작 절차(설정 적용·엔진 준비) 뒤 한 번 부른다.
/// 시작할 때 읽어 둔 위치부터 이어 가고, 이어 갈 수 없으면 첫 방 테마로 시작한다.
pub fn api_resume_show() -> Result<(), AtmosError> {
    crate::core::show_state::resume(crate::core::app_signals::now_ms())
}

/// 첫 방 테마 시작: 전체 정지 → 설정의 첫 방을 활성으로 → 그 방 루프 재생. OSC 테마 시작과 같다.
pub fn api_theme_start() -> Result<(), AtmosError> {
    api_stop_all()?;
    let first_room = GLOBAL_STATE
        .config
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .and_then(|config| config.rooms.first())
        .map(|room| {
            let loops: Vec<String> = room.tracks.iter().filter(|t| t.is_loop).map(|t| t.id.clone()).collect();
            (room.id.clone(), loops)
        });
    let Some((room_id, loop_ids)) = first_room else {
        return Ok(());
    };
    api_set_active_room(Some(room_id.clone()))?;
    for track_id in loop_ids {
        if let Err(e) = api_play_track(room_id.clone(), track_id.clone()) {
            GLOBAL_STATE.log(format!("테마 시작: {track_id} 재생 실패: {}", e.message));
        }
    }
    Ok(())
}
