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

/// 테마 시작. 대시보드 Start 버튼, OSC 테마 시작, 감시가 다시 띄운 앱이 공연을 이어 갈 수 없을 때가 모두 이 함수
/// 하나를 부른다(사용자 결정 2026-10-12: OSC 테마 시작 = Start 버튼). 전체 정지 뒤, 전시 모드면 모든 방의 루프 트랙을,
/// 아니면 첫 방을 활성 방으로 하고 그 방의 루프 트랙을 튼다. 그 밖의 트랙은 OSC 신호로 틀고, 전시 모드가 아니면 방
/// 클리어 때 다음 방 루프가 이어진다(osc::listener). 루프 트랙 재생이 실패해도 나머지는 다 틀고, 실패한 트랙을 모아
/// Err로 알린다(앱 로그에도 남긴다). 방이 없으면 정지만 한다.
pub fn api_theme_start() -> Result<(), AtmosError> {
    api_stop_all()?;
    let plan = GLOBAL_STATE.config.read().unwrap_or_else(|e| e.into_inner()).as_ref().map(|config| {
        let rooms = if config.is_exhibition_mode { config.rooms.len() } else { 1 };
        let loops: Vec<(String, String, String)> = config
            .rooms
            .iter()
            .take(rooms)
            .flat_map(|room| {
                room.tracks.iter().filter(|t| t.is_loop).map(|t| (room.id.clone(), t.id.clone(), t.name.clone()))
            })
            .collect();
        (config.is_exhibition_mode, config.rooms.first().map(|room| room.id.clone()), loops)
    });
    let Some((exhibition, Some(first_room), loops)) = plan else {
        return Ok(());
    };
    if !exhibition {
        api_set_active_room(Some(first_room))?;
    }
    let mut failed = Vec::new();
    for (room_id, track_id, name) in loops {
        if let Err(e) = api_play_track(room_id, track_id) {
            GLOBAL_STATE.log(format!("테마 시작: {name} 재생 실패: {}", e.message));
            failed.push(format!("{name}: {}", e.message));
        }
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(AtmosError { message: format!("루프 트랙 {}개 재생 실패 — {}", failed.len(), failed.join(" / ")) })
    }
}
