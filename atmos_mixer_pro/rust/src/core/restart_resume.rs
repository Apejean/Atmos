//! 엔진이 스스로 재시작할 때(워치독 콜백 공백, 장치 오류·재설정, 장치 목록 변화) 재생 중이던
//! 트랙을 새 엔진에서 멈춘 위치부터 다시 튼다.
//!
//! 순서는 다음과 같다.
//! 1. 감시 루프가 재시작을 결정한 직후 `take_snapshot`을 부른다.
//! 2. 옛 엔진을 drop하고 새 엔진을 띄운다.
//! 3. `resume_after_resync`가 Dart에 재시작을 알리고, 재동기화 완료를 기다린 뒤 재개한다.
//!
//! 기다리는 사이 사용자가 정지하면(트랙 정지, 전체 정지, 방 비우기 — 대시보드·OSC 모두) 그
//! 트랙은 다시 틀지 않는다(`cancel_*`). 재개 전에 또 재시작하면 남은 항목은 다음 재개로 넘어간다.
use crate::audio::engine::ENGINE_GENERATION;
use crate::audio::playback_cursor::CURSOR_TABLE;
use crate::common::commands::AudioCommand;
use crate::core::state::GLOBAL_STATE;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub struct ResumeEntry {
    pub room_id: String,
    pub track_id: String,
    pub seconds: f64,
}

/// 재개 대기 항목과 그것을 뜬 순간(단조 밀리초, `GLOBAL_STATE.monotonic_ms`). 재개할 때 그 사이 흐른 시간만큼
/// 건너뛴다(유지 스레드 설계 D12·U2).
#[derive(Debug, Clone)]
struct Pending {
    entry: ResumeEntry,
    taken_ms: u64,
}

static PENDING: Mutex<Vec<Pending>> = Mutex::new(Vec::new());
/// 새 엔진이 준비된 자기 재시작의 순번. 장치 이벤트 스트림이 "EngineRestarted:<순번>"으로 알린다.
pub static RESTART_SEQ: AtomicU32 = AtomicU32::new(0);
/// Dart가 재동기화를 마쳤다고 알린 가장 큰 순번.
pub static RESYNC_ACK: AtomicU32 = AtomicU32::new(0);
/// 자기 재시작이 새 엔진을 여는 동안 켠다. 그때 만든 믹서는 부팅 뮤트 램프를 짧게 쓴다(audio::mixer::RESTART_RAMP_S).
static SHORT_STARTUP_RAMP: AtomicBool = AtomicBool::new(false);

/// 자기 재시작 스레드가 새 엔진을 열기 전에 켜고, 다 연 뒤(비상 전환 포함) 끈다.
pub fn set_short_startup_ramp(on: bool) {
    SHORT_STARTUP_RAMP.store(on, Ordering::SeqCst);
}

/// 지금 만드는 믹서가 자기 재시작의 것인가(`AudioMixer::new`가 묻는다).
pub fn short_startup_ramp() -> bool {
    SHORT_STARTUP_RAMP.load(Ordering::SeqCst)
}
/// Dart 재동기화를 기다리는 최대 시간. 넘기면 그대로 재개한다(무음보다 낫다).
pub const RESYNC_TIMEOUT: Duration = Duration::from_secs(2);
/// 완료 신호 뒤 재개까지 둔다. Dart 재동기화 호출은 응답을 기다리지 않고 FRB 워커 스레드에서 순서 없이
/// 실행되므로, 신호가 마지막 상태 명령보다 먼저 닿을 수 있다. 재개는 300ms 페이드 인으로 시작한다.
const SETTLE_AFTER_ACK: Duration = Duration::from_millis(100);

fn pending() -> MutexGuard<'static, Vec<Pending>> {
    PENDING.lock().unwrap_or_else(|e| e.into_inner())
}

/// 감시 루프가 자기 재시작을 결정한 직후, 옛 엔진을 drop하기 전에 부른다(`generation`은 그 엔진의 세대).
/// 재생 목록과 커서 표로 재개 항목을 만들어 대기열에 더하고, 재생 목록과 미처리 명령을 비운다.
/// 그사이 사용자가 엔진을 직접 멈추거나 다시 띄웠으면(세대가 바뀜) 아무것도 하지 않는다. 공개 진입점은
/// 세대를 먼저 올린 뒤 대기를 버리므로, 세대 확인과 대기열 추가가 한 잠금 안에 있어야 낡은 항목이 남지 않는다.
/// 큐에 남은 재생 명령은 옛 믹서가 아직 가져가지 않은 것이라 0초부터 재개 항목에 넣고 큐에서 뺀다 —
/// 남겨 두면 새 엔진에서 두 번 재생된다. 상태 명령은 Dart 재동기화가 다시 보낸다. 위치도 없고 큐에도
/// 없는 인스턴스는 이미 끝났는데 목록 정리(GC 스레드)만 늦은 것이라 다시 틀지 않는다.
/// 커서 표는 지우지 않는다. 옛 콜백이 drop 전까지 계속 쓰므로 여기서 지우면 칸마다 쓰는 쪽이 둘이
/// 된다. 새 엔진의 `AudioMixer::new`가 지우고, 그 전까지 남은 칸은 재생 목록에 없어 조회에 안 보인다.
pub fn take_snapshot(generation: u64) {
    let mut queue = pending();
    if ENGINE_GENERATION.load(Ordering::SeqCst) != generation {
        return;
    }
    let mut queued = HashSet::new();
    while let Ok(cmd) = GLOBAL_STATE.command_receiver.try_recv() {
        if let AudioCommand::PlayTrack { instance, .. } = &cmd {
            queued.insert(instance.instance_id);
        }
    }
    let playing = GLOBAL_STATE.playing_track_ids.read().unwrap_or_else(|e| e.into_inner()).clone();
    let cursors: HashMap<u64, f64> = CURSOR_TABLE.snapshot().into_iter().collect();
    let config = GLOBAL_STATE.config.read().unwrap_or_else(|e| e.into_inner()).clone();
    let mut entries: Vec<ResumeEntry> = playing
        .iter()
        .filter_map(|(instance_id, track_id)| {
            let seconds = match cursors.get(instance_id) {
                Some(s) => *s,
                None if queued.contains(instance_id) => 0.0,
                None => return None,
            };
            let room = config
                .as_ref()?
                .rooms
                .iter()
                .find(|r| r.tracks.iter().any(|t| &t.id == track_id))?;
            Some(ResumeEntry { room_id: room.id.clone(), track_id: track_id.clone(), seconds })
        })
        .collect();
    entries.sort_by(|a, b| a.track_id.cmp(&b.track_id).then(a.seconds.total_cmp(&b.seconds)));
    let taken_ms = GLOBAL_STATE.monotonic_ms();
    queue.extend(entries.into_iter().map(|entry| Pending { entry, taken_ms }));
    drop(queue);

    GLOBAL_STATE.clear_playing_tracks();
}

// 아래 cancel_*는 대기열 잠금 안에서 정지 순번을 올린다. 재개 쪽(resume_pending)도 같은 잠금 안에서 순번을 읽고
// 대기열을 꺼내므로, 정지는 "꺼내기 전(대기열이 비어 아무것도 안 틂)"이나 "꺼낸 뒤(순번이 바뀌어 그룹을 시작하지
// 않음)" 둘 중 하나로만 보인다. 새 순번을 돌려준다(그 뒤 바로 그룹을 트는 쪽이 기준으로 쓴다).

/// 사용자가 전체 정지하거나 엔진을 직접 다시 띄우면 재개 대기를 버린다. 준비 중인 그룹 시작도 멈춘다(정지 순번).
pub fn cancel_all() -> u64 {
    let mut queue = pending();
    let epoch = crate::core::group_start::note_stop();
    queue.clear();
    epoch
}

/// 사용자가 트랙을 정지하면 그 트랙은 재개하지 않는다. 준비 중인 그룹 시작도 멈춘다(정지 순번).
pub fn cancel_track(track_id: &str) -> u64 {
    let mut queue = pending();
    let epoch = crate::core::group_start::note_stop();
    queue.retain(|p| p.entry.track_id != track_id);
    epoch
}

/// 사용자가 방을 비우면(룸 전환) 그 방의 트랙은 재개하지 않는다. 준비 중인 그룹 시작도 멈춘다(정지 순번).
pub fn cancel_room(room_id: &str) -> u64 {
    let mut queue = pending();
    let epoch = crate::core::group_start::note_stop();
    queue.retain(|p| p.entry.room_id != room_id);
    epoch
}

/// 지금 재개를 기다리는 항목(테스트·진단용).
pub fn pending_entries() -> Vec<ResumeEntry> {
    pending().iter().map(|p| p.entry.clone()).collect()
}

/// 끊긴 시간만큼 건너뛴 재개 항목(순수 함수, 유지 스레드 설계 6절). `info`는 트랙 id로 (반복인가, 파일 길이 초)를
/// 돌려준다. 길이를 모르면 그 항목은 건너뛰지 않는다. 반복은 길이의 나머지로, 반복이 아닌 트랙은 끝을 넘으면 뺀다.
pub fn skip_outage(
    entries: Vec<(ResumeEntry, f64)>,
    info: impl Fn(&str) -> Option<(bool, f64)>,
) -> Vec<ResumeEntry> {
    entries
        .into_iter()
        .filter_map(|(mut entry, outage_s)| {
            let Some((is_loop, length_s)) = info(&entry.track_id).filter(|(_, len)| *len > 0.0) else {
                return Some(entry);
            };
            let moved = entry.seconds + outage_s.max(0.0);
            if is_loop {
                entry.seconds = moved % length_s;
                Some(entry)
            } else if moved >= length_s {
                None
            } else {
                entry.seconds = moved;
                Some(entry)
            }
        })
        .collect()
}

/// 설정에서 트랙이 반복인지와 파일 길이(초, 파일 머리에서 읽음)를 찾는다. 오디오 스레드 밖에서만 부른다.
fn track_length_info(track_id: &str) -> Option<(bool, f64)> {
    let (is_loop, path) = {
        let config = GLOBAL_STATE.config.read().unwrap_or_else(|e| e.into_inner());
        let track = config.as_ref()?.rooms.iter().flat_map(|r| r.tracks.iter()).find(|t| t.id == track_id)?;
        (track.is_loop, track.file_path.clone())
    };
    let (frames, rate) = crate::audio::streaming::file_frames(&path)?;
    (rate > 0).then(|| (is_loop, frames as f64 / f64::from(rate)))
}

/// Dart가 `seq`번 재시작의 재동기화를 마쳤다.
pub fn ack(seq: u32) {
    RESYNC_ACK.fetch_max(seq, Ordering::AcqRel);
}

/// `seq`번 완료 신호를 `timeout`까지 기다린다. 받았으면 true.
pub fn wait_for_ack(seq: u32, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while RESYNC_ACK.load(Ordering::Acquire) < seq {
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    true
}

/// 대기열을 지금 재생한다. 실패한 항목(설정에서 사라진 트랙 등)은 기록하고 건너뛴다.
/// 대기열을 한 번에 꺼내 한 그룹으로 같은 샘플에 함께 시작한다(그룹 시작 설계 3.5). 위치는 한 콜백의 커서 표에서
/// 나왔으므로 같은 그룹 안에서 맞고, 길이가 같은 트랙은 가장 앞선 위치로 맞춘다(그룹 시작 전 버전이 남긴 어긋남).
/// 예전에는 한 항목씩 잠금을 쥔 채 꺼내 재생해, 통째로 꺼낸 뒤 그 사이 들어온 정지·전체 정지(Emergency) 뒤에 남은
/// 항목이 재생되는 것을 막았다. 그룹은 모든 트랙을 준비한 뒤 함께 시작하므로 잠금을 쥔 채 기다리면 비상 정지가 그만큼
/// 늦어진다. 대신 준비하는 사이 정지가 들어오면 정지 순번(group_start::STOP_EPOCH)이 바뀌어 그룹을 시작하지 않고
/// 정리하므로, 정지 명령 뒤에 소리가 나는 일은 여전히 없다.
pub fn resume_pending() {
    let (waiting, epoch) = {
        let mut queue = pending();
        (std::mem::take(&mut *queue), crate::core::group_start::current_epoch())
    };
    if waiting.is_empty() {
        return;
    }
    // 뜬 뒤 흐른 시간만큼 건너뛴다. 같은 순간에 뜬 항목은 같은 시간을 더하므로 그룹 안 상대 위치가 지켜진다.
    let now_ms = GLOBAL_STATE.monotonic_ms();
    let entries = skip_outage(
        waiting
            .into_iter()
            .map(|p| {
                let outage_s = now_ms.saturating_sub(p.taken_ms) as f64 / 1000.0;
                (p.entry, outage_s)
            })
            .collect(),
        track_length_info,
    );
    if entries.is_empty() {
        return;
    }
    let mut plan: Vec<(String, String, f64)> =
        entries.into_iter().map(|e| (e.room_id, e.track_id, e.seconds)).collect();
    crate::core::group_start::align_plan(&mut plan);
    for (track_id, err) in crate::core::group_start::play_group_since(&plan, epoch) {
        GLOBAL_STATE.log(format!("재시작 복원: {track_id} 재개 실패: {}", err.message));
    }
}

/// 새 엔진이 준비된 뒤 재기동 스레드에서 부른다. Dart에 재시작을 알리고 재동기화 완료(최대 2초)를
/// 기다린 뒤 재개한다. 기다리는 사이 세대가 바뀌면(사용자 재기동, 또 다른 재시작) 재개하지 않는다 —
/// 사용자 재기동은 대기열을 이미 버렸고, 또 다른 재시작은 대기열을 이어받는다.
pub fn resume_after_resync() {
    let generation = ENGINE_GENERATION.load(Ordering::SeqCst);
    let seq = RESTART_SEQ.fetch_add(1, Ordering::AcqRel) + 1;
    if wait_for_ack(seq, RESYNC_TIMEOUT) {
        std::thread::sleep(SETTLE_AFTER_ACK);
    }
    if ENGINE_GENERATION.load(Ordering::SeqCst) != generation {
        return;
    }
    resume_pending();
}
