//! 그룹 시작(docs/superpowers/specs/2026-11-12-group-start-design.md, HANDOFF 확정된 결정 "동기").
//!
//! 여러 트랙을 한 번에 트는 네 경로(테마 시작, OSC 방 비우기 뒤 다음 방 루프, 앱 재실행 이어 가기, 엔진 재시작 이어
//! 가기)가 쓰는 준비 쪽이다. 오디오 스레드 밖(FRB 워커·OSC·재기동 스레드)에서만 부른다.
//! 1. 트랙마다 지금처럼 인스턴스를 만들되 그룹 번호를 붙여 대기(armed)로 보낸다(`api::simple::play_track_from_in`).
//! 2. 디스크 트랙의 첫 묶음이 모두 준비될 때까지 기다린다(상한 [`PREROLL_TIMEOUT`]).
//! 3. `StartGroup` 하나를 보낸다. 믹서가 그 블록의 첫 샘플에서 모두 함께 시작한다(audio::mixer `start_group`).
//!
//! 준비하는 사이에 사용자 정지(트랙·방·전체, 사용자 엔진 재기동)가 끼어들면 [`STOP_EPOCH`]이 바뀐다. 그러면 남은 트랙은
//! 보내지 않고, 시작하지 않고 `CancelGroup`으로 대기 인스턴스를 정리한다. 정지가 언제나 이긴다. 트랙 하나를 멈춰도
//! 준비 중인 그룹 전체를 시작하지 않는다(창이 수 ms라 드물고, 멈춘 것이 다시 나는 쪽보다 안전하다).
use crate::api::error::AtmosError;
use crate::common::commands::AudioCommand;
use crate::core::state::GLOBAL_STATE;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 첫 묶음을 기다리는 상한. 넘으면 준비된 트랙만으로 먼저 시작하고, 늦은 트랙은 첫 묶음이 오는 대로 밀린 만큼
/// 건너뛰어 그룹 위치로 합류한다(설계 3.4).
pub const PREROLL_TIMEOUT: Duration = Duration::from_millis(1000);

/// 그룹 번호. 0은 "그룹 아님"이라 1부터 쓴다.
static NEXT_GROUP_ID: AtomicU64 = AtomicU64::new(1);

/// 사용자 정지 순번. 트랙·방·전체 정지와 사용자 엔진 재기동마다 올라간다(core::restart_resume의 cancel_*가 부른다).
pub static STOP_EPOCH: AtomicU64 = AtomicU64::new(0);

/// 그룹 준비(트랙 하나의 재생 명령 보내기)와 정지 순번 올리기를 한 줄로 세운다. 정지 쪽은 이 잠금을 잡고 순번을
/// 올리고, 정지 명령(StopAll 등)은 그다음에 보낸다. 준비 쪽은 트랙마다 이 잠금 안에서 순번을 확인한 뒤 보내므로
/// 정지 명령 뒤에 재생 명령이 남지 않고 재생 목록도 비어 있다(예전 이어 가기의 "한 항목씩 잠금" 규칙,
/// test_restart_resume_races). 첫 묶음을 기다리는 동안에는 잡지 않는다(비상 정지가 늦어지지 않게).
static PREP: Mutex<()> = Mutex::new(());

/// 사용자 정지가 있었다고 적고 새 순번을 돌려준다. 준비 중인 그룹은 남은 트랙을 보내지 않고 시작하지 않는다.
/// 정지 뒤에 바로 그룹을 트는 쪽(테마 시작, OSC 방 비우기 뒤 다음 방)은 이 값을 기준으로 넘긴다.
pub fn note_stop() -> u64 {
    let _prep = PREP.lock().unwrap_or_else(|e| e.into_inner());
    STOP_EPOCH.fetch_add(1, Ordering::SeqCst) + 1
}

/// 지금 정지 순번. 그룹을 계획하는 순간에 읽어 [`play_group_since`]에 넘긴다.
pub fn current_epoch() -> u64 {
    STOP_EPOCH.load(Ordering::SeqCst)
}

/// 한 그룹을 준비하는 동안 모으는 것.
pub struct GroupStart {
    pub id: u64,
    /// 디스크 트랙마다 "첫 묶음을 링에 넣었다" 표시(DiskStreamer). 메모리 트랙은 처음부터 준비돼 있어 없다.
    pub ready: Vec<Arc<AtomicBool>>,
}

impl GroupStart {
    pub fn new() -> Self {
        Self { id: NEXT_GROUP_ID.fetch_add(1, Ordering::SeqCst), ready: Vec::new() }
    }
}

impl Default for GroupStart {
    fn default() -> Self {
        Self::new()
    }
}

/// `plan`(방 id, 트랙 id, 시작 위치 초)의 트랙을 한 그룹으로 같은 샘플에 함께 시작한다. 이미 재생 중인 루프는
/// 지금처럼 다시 틀지 않는다. 실패한 트랙은 (트랙 id, 오류)로 돌려주고 나머지는 그대로 시작한다.
/// 기준 정지 순번은 지금 값이다. 계획을 만든 뒤 이 함수를 부르기 전에 정지가 끼어들 수 있으면 [`play_group_since`]를 쓴다.
pub fn play_group(plan: &[(String, String, f64)]) -> Vec<(String, AtmosError)> {
    play_group_since(plan, current_epoch())
}

/// [`play_group`]과 같고, `epoch`(계획을 만든 순간의 정지 순번) 뒤에 들어온 정지가 있으면 시작하지 않는다.
pub fn play_group_since(plan: &[(String, String, f64)], epoch: u64) -> Vec<(String, AtmosError)> {
    let mut failed = Vec::new();
    if plan.is_empty() {
        return failed;
    }
    crate::core::event_log::record_request("play_group", &format!("{}tracks", plan.len()));
    let mut group = GroupStart::new();
    for (room_id, track_id, seconds) in plan {
        let _prep = PREP.lock().unwrap_or_else(|e| e.into_inner());
        if STOP_EPOCH.load(Ordering::SeqCst) != epoch {
            break; // 정지가 들어왔다: 남은 트랙은 보내지 않는다
        }
        if let Err(e) =
            crate::api::simple::play_track_from_in(room_id.clone(), track_id.clone(), *seconds, Some(&mut group))
        {
            failed.push((track_id.clone(), e));
        }
    }
    // 프리롤: 디스크 트랙의 첫 묶음을 기다린다. 정지가 끼어들면 기다리지 않는다.
    let deadline = Instant::now() + PREROLL_TIMEOUT;
    while !group.ready.iter().all(|r| r.load(Ordering::Acquire))
        && Instant::now() < deadline
        && STOP_EPOCH.load(Ordering::SeqCst) == epoch
    {
        std::thread::sleep(Duration::from_millis(1));
    }
    let not_ready = group.ready.iter().filter(|r| !r.load(Ordering::Acquire)).count();
    if not_ready > 0 {
        GLOBAL_STATE.log(format!(
            "그룹 시작: {}개 중 {not_ready}개는 {}ms 안에 첫 묶음이 오지 않아 먼저 시작하고, 늦은 트랙은 그룹 위치로 합류한다",
            group.ready.len(),
            PREROLL_TIMEOUT.as_millis()
        ));
    }
    let cmd = if STOP_EPOCH.load(Ordering::SeqCst) == epoch {
        AudioCommand::StartGroup { group_id: group.id }
    } else {
        GLOBAL_STATE.log("그룹 시작: 준비하는 사이 정지가 들어와 시작하지 않는다".to_string());
        AudioCommand::CancelGroup { group_id: group.id }
    };
    let _ = GLOBAL_STATE.command_sender.send(cmd);
    failed
}

/// 같은 그룹에서 파일 길이(프레임 수, 샘플레이트)가 같은 트랙들은 위치를 그중 가장 앞선 위치 하나로 맞춘다(설계 3.5).
/// 그룹 시작 전 버전이 저장한 위치는 1~5블록 어긋나 있을 수 있다(진단 T252·T264). 한 작품을 채널별로 나눈 파일은
/// 길이가 같으므로 함께 맞춘다. 길이를 모르거나 다른 트랙은 그대로 둔다.
pub fn align_same_length(seconds: &mut [f64], lengths: &[Option<(u64, u32)>]) {
    for i in 0..seconds.len().min(lengths.len()) {
        let Some(len) = lengths[i] else { continue };
        let mut lead = seconds[i];
        for j in 0..seconds.len().min(lengths.len()) {
            if lengths[j] == Some(len) && seconds[j] > lead {
                lead = seconds[j];
            }
        }
        seconds[i] = lead;
    }
}

/// 이어 가기 계획의 트랙 파일 길이를 설정에서 찾아 [`align_same_length`]로 맞춘다.
pub fn align_plan(plan: &mut [(String, String, f64)]) {
    let paths: Vec<Option<String>> = {
        let guard = GLOBAL_STATE.config.read().unwrap_or_else(|e| e.into_inner());
        plan.iter()
            .map(|(room_id, track_id, _)| {
                guard.as_ref().and_then(|c| {
                    c.rooms
                        .iter()
                        .find(|r| &r.id == room_id)
                        .and_then(|r| r.tracks.iter().find(|t| &t.id == track_id))
                        .map(|t| t.file_path.clone())
                })
            })
            .collect()
    };
    let lengths: Vec<Option<(u64, u32)>> =
        paths.iter().map(|p| p.as_deref().and_then(crate::audio::streaming::file_frames)).collect();
    let mut seconds: Vec<f64> = plan.iter().map(|p| p.2).collect();
    align_same_length(&mut seconds, &lengths);
    for (p, s) in plan.iter_mut().zip(seconds) {
        p.2 = s;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 길이가_같은_트랙은_가장_앞선_위치로_맞춘다() {
        let mut s = [10.0, 10.021, 9.979, 3.0, 7.0];
        let len = Some((14_400_000, 48_000));
        let other = Some((9_600_000, 48_000));
        align_same_length(&mut s, &[len, len, len, other, None]);
        assert_eq!(s, [10.021, 10.021, 10.021, 3.0, 7.0]);
    }

    #[test]
    fn 샘플레이트가_다르면_같은_길이로_보지_않는다() {
        let mut s = [1.0, 2.0];
        align_same_length(&mut s, &[Some((480_000, 48_000)), Some((480_000, 44_100))]);
        assert_eq!(s, [1.0, 2.0]);
    }

    #[test]
    fn 그룹_번호는_0이_아니고_매번_다르다() {
        let (a, b) = (GroupStart::new(), GroupStart::new());
        assert_ne!(a.id, 0);
        assert_ne!(a.id, b.id);
    }
}
