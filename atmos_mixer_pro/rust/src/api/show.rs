//! 공연 시작·이어 가기(docs/superpowers/specs/2026-10-05-crash-relaunch-supervisor-design.md 4.4).
use crate::api::error::AtmosError;
use crate::api::simple::{api_set_active_room, stop_all};
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
/// 루프들은 한 그룹으로 같은 샘플에 함께 시작한다(core::group_start, 그룹 시작 설계). 예전에는 하나씩 틀어, 보내는
/// 도중 콜백 경계가 지나가면 뒤 트랙들이 한 블록(1024프레임에서 21.3ms) 늦게 시작한 채로 남았다(진단 T252).
pub fn api_theme_start() -> Result<(), AtmosError> {
    // 장치를 기다리거나 실패한 동안에는 시작하지 않는다(유지 스레드 설계 D11). 전체 정지도 하지 않아 지금 상태를 바꾸지 않는다.
    if !crate::core::engine_keeper::accepts_play() {
        return Err(AtmosError {
            message: format!(
                "{} ({})",
                crate::osc::reply::ENGINE_WAITING_PREFIX,
                crate::core::engine_keeper::state().name()
            ),
        });
    }
    // 이 정지가 올린 순번이 기준이다. 그 뒤 루프를 준비하는 사이 다른 정지(Emergency 등)가 들어오면 시작하지 않는다.
    let epoch = stop_all()?;
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
        // 대시보드 방 카드의 잠금 표시. 버튼으로 시작하든 OSC로 시작하든 같은 상태가 화면에 간다(다음 알림에 실림).
        GLOBAL_STATE.set_theme_started(true);
        api_set_active_room(Some(first_room))?;
    }
    let plan: Vec<(String, String, f64)> =
        loops.iter().map(|(room_id, track_id, _)| (room_id.clone(), track_id.clone(), 0.0)).collect();
    let mut failed = Vec::new();
    for (track_id, e) in crate::core::group_start::play_group_since(&plan, epoch) {
        let name = loops.iter().find(|(_, t, _)| *t == track_id).map_or(track_id.as_str(), |(_, _, n)| n.as_str());
        GLOBAL_STATE.log(format!("테마 시작: {name} 재생 실패: {}", e.message));
        failed.push(format!("{name}: {}", e.message));
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(AtmosError { message: format!("루프 트랙 {}개 재생 실패 — {}", failed.len(), failed.join(" / ")) })
    }
}
