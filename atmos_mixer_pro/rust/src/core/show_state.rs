//! 공연 상태 저장과 이어 가기(docs/superpowers/specs/2026-10-05-crash-relaunch-supervisor-design.md 4.4),
//! 재생 중 무음 경고(4.5). 전용 스레드가 5초마다 재생 중인 트랙과 위치를 show_state.json에 저장하고 무음을 본다.
//! 앱이 시작할 때 지난 상태를 먼저 읽어 두므로, 저장 스레드가 파일을 덮어써도 이어 가기는 지난 상태를 쓴다.
//! 오디오 스레드가 적어 둔 원자 값(피크, All Mute)을 읽기만 한다(DSP 3법칙).
use crate::api::error::AtmosError;
use crate::audio::playback_cursor::CURSOR_TABLE;
use crate::common::config::AppConfig;
use crate::core::state::GLOBAL_STATE;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const SHOW_STATE_FILE: &str = "show_state.json";
/// 저장 간격. 이어 갈 때 위치가 최대 이만큼 앞이다.
pub const SAVE_INTERVAL: Duration = Duration::from_secs(5);
/// 저장한 지 이보다 오래면 이어 가지 않고 첫 방 테마로 시작한다(오래 꺼져 있던 뒤에는 처음부터가 안전하다).
pub const MAX_RESUME_AGE_MS: u64 = 10 * 60 * 1000;
/// 저장 시각이 지금보다 이만큼 넘게 앞서면(시계가 뒤로 감) 믿지 않는다.
const CLOCK_SKEW_MS: u64 = 60 * 1000;
/// 무음이 이만큼 이어지면 경고한다.
pub const SILENCE_WARN_AFTER_MS: u64 = 60 * 1000;
/// 무음이 이어지면 이 간격으로 다시 경고한다.
pub const SILENCE_REPEAT_MS: u64 = 10 * 60 * 1000;

/// 저장한 트랙 하나. 루프는 한 바퀴 안의 위치다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedTrack {
    pub room_id: String,
    pub track_id: String,
    pub seconds: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShowState {
    /// 저장 시각(벽시계 밀리초). 재부팅을 넘어 비교하므로 벽시계를 쓴다.
    pub saved_at_ms: u64,
    pub active_room_id: Option<String>,
    pub tracks: Vec<SavedTrack>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResumePlan {
    /// 저장된 위치부터 이어 간다(활성 방, 지금 설정에 남은 트랙).
    Resume { active_room_id: Option<String>, tracks: Vec<SavedTrack> },
    /// 이어 갈 수 없어 첫 방 테마로 시작한다(이유).
    ThemeStart(String),
}

/// 이어 갈지 정한다.
pub fn decide_resume(saved: Option<&ShowState>, now_ms: u64, config: Option<&AppConfig>) -> ResumePlan {
    let Some(saved) = saved else {
        return ResumePlan::ThemeStart("저장된 공연 상태가 없다".into());
    };
    if saved.saved_at_ms > now_ms.saturating_add(CLOCK_SKEW_MS) {
        return ResumePlan::ThemeStart("저장 시각이 지금보다 뒤다(시계가 바뀜)".into());
    }
    let age_ms = now_ms.saturating_sub(saved.saved_at_ms);
    if age_ms > MAX_RESUME_AGE_MS {
        return ResumePlan::ThemeStart(format!("저장한 지 {}초가 지났다(10분 넘음)", age_ms / 1000));
    }
    if saved.tracks.is_empty() {
        return ResumePlan::ThemeStart("재생 중이던 트랙이 없었다".into());
    }
    let tracks: Vec<SavedTrack> = saved
        .tracks
        .iter()
        .filter(|t| {
            config.is_some_and(|c| {
                c.rooms.iter().any(|r| r.id == t.room_id && r.tracks.iter().any(|x| x.id == t.track_id))
            })
        })
        .cloned()
        .collect();
    if tracks.is_empty() {
        return ResumePlan::ThemeStart("저장된 트랙이 지금 설정에 없다".into());
    }
    let active_room_id = saved
        .active_room_id
        .clone()
        .filter(|id| config.is_some_and(|c| c.rooms.iter().any(|r| &r.id == id)));
    ResumePlan::Resume { active_room_id, tracks }
}

/// 지금 공연 상태. 위치를 아는 인스턴스만 넣는다(막 큐에 들어갔거나 이미 끝난 것은 뺀다).
/// 엔진 재시작 복원을 기다리는 트랙도 넣는다 — 재시작 중에는 재생 목록이 잠깐 비어 있다.
pub fn capture(now_ms: u64) -> ShowState {
    let playing = GLOBAL_STATE.playing_track_ids.read().unwrap_or_else(|e| e.into_inner()).clone();
    let cursors: HashMap<u64, f64> = CURSOR_TABLE.snapshot().into_iter().collect();
    let active_room_id = GLOBAL_STATE.active_room_id.read().unwrap_or_else(|e| e.into_inner()).clone();
    let mut tracks: Vec<SavedTrack> = {
        let config = GLOBAL_STATE.config.read().unwrap_or_else(|e| e.into_inner());
        playing
            .iter()
            .filter_map(|(instance_id, track_id)| {
                let seconds = *cursors.get(instance_id)?;
                let room = config.as_ref()?.rooms.iter().find(|r| r.tracks.iter().any(|t| &t.id == track_id))?;
                Some(SavedTrack { room_id: room.id.clone(), track_id: track_id.clone(), seconds })
            })
            .collect()
    };
    tracks.extend(crate::core::restart_resume::pending_entries().into_iter().map(|e| SavedTrack {
        room_id: e.room_id,
        track_id: e.track_id,
        seconds: e.seconds,
    }));
    // 장치를 기다리며 미뤄 둔 이어 가기도 넣는다. 그동안 파일을 빈 목록으로 덮으면 그 사이 앱이 다시 켜졌을 때
    // 이어 갈 것이 없다(④ 실기 F2). 그 계획의 활성 방이 이어 간 뒤의 활성 방이다.
    let mut active_room_id = active_room_id;
    if let Some((room, held)) = deferred_tracks() {
        active_room_id = room;
        tracks.extend(held);
    }
    tracks.sort_by(|a, b| a.track_id.cmp(&b.track_id).then(a.seconds.total_cmp(&b.seconds)));
    ShowState { saved_at_ms: now_ms, active_room_id, tracks }
}

pub fn save(dir: &Path, state: &ShowState) -> std::io::Result<()> {
    let json = serde_json::to_string(state).map_err(std::io::Error::other)?;
    crate::core::app_signals::atomic_write(&dir.join(SHOW_STATE_FILE), &json)
}

/// 저장 파일을 읽는다. 없으면 Ok(None), 깨졌으면 Err.
pub fn load(dir: &Path) -> Result<Option<ShowState>, serde_json::Error> {
    let Ok(text) = std::fs::read_to_string(dir.join(SHOW_STATE_FILE)) else {
        return Ok(None);
    };
    serde_json::from_str(&text).map(Some)
}

/// 앱이 시작할 때 읽어 둔 지난 공연 상태. 이어 가기가 한 번 꺼내 쓴다.
static PREVIOUS: Mutex<Option<ShowState>> = Mutex::new(None);

/// 지난 공연 상태를 읽어 두고 저장 스레드를 시작한다(앱 시작 때 한 번, 두 번째부터는 무시).
/// 첫 저장은 [interval] 뒤라 읽기가 먼저다.
pub fn start(dir: PathBuf, interval: Duration) {
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        let previous = match load(&dir) {
            Ok(previous) => previous,
            Err(e) => {
                GLOBAL_STATE.log(format!("공연 상태 파일이 깨져 있어 쓰지 않는다: {e}"));
                None
            }
        };
        *PREVIOUS.lock().unwrap_or_else(|e| e.into_inner()) = previous;
        let spawned = std::thread::Builder::new()
            .name("show-state".into())
            .spawn(move || saver_loop(&dir, interval));
        if let Err(e) = spawned {
            GLOBAL_STATE.log(format!("공연 상태 저장 스레드를 시작하지 못했다: {e}"));
        }
    });
}

fn saver_loop(dir: &Path, interval: Duration) {
    let started = Instant::now();
    let mut silence = SilenceWatch::default();
    let mut failing = false;
    loop {
        std::thread::sleep(interval);
        match save(dir, &capture(crate::core::app_signals::now_ms())) {
            Ok(()) => failing = false,
            Err(e) => {
                if !failing {
                    GLOBAL_STATE.log(format!("공연 상태를 저장하지 못했다(다시 시도한다): {e}"));
                }
                failing = true;
            }
        }
        if let Some(count) = check_silence(&mut silence, started.elapsed().as_millis() as u64) {
            GLOBAL_STATE.log(format!(
                "무음 경고({count}회째): 루프를 재생 중이고 All Mute가 꺼져 있는데 출력이 1분 넘게 완전히 0이다. \
                 라우팅·장치·볼륨을 확인하세요"
            ));
        }
    }
}

/// 재생 중 무음 판단. 무음 조건이 60초 이어지면 경고하고, 계속 이어지면 10분마다 다시 경고한다.
/// 조건이 풀리면(소리가 남, All Mute, 루프 정지) 처음부터 다시 잰다.
#[derive(Debug, Default)]
pub struct SilenceWatch {
    silent_since_ms: Option<u64>,
    last_warned_ms: Option<u64>,
}

impl SilenceWatch {
    /// [now_ms]는 단조 시계(밀리초)다. 이번에 경고해야 하면 true.
    pub fn observe(&mut self, now_ms: u64, silent: bool) -> bool {
        if !silent {
            *self = SilenceWatch::default();
            return false;
        }
        let since = *self.silent_since_ms.get_or_insert(now_ms);
        if now_ms.saturating_sub(since) < SILENCE_WARN_AFTER_MS {
            return false;
        }
        if self.last_warned_ms.is_some_and(|warned| now_ms.saturating_sub(warned) < SILENCE_REPEAT_MS) {
            return false;
        }
        self.last_warned_ms = Some(now_ms);
        true
    }
}

/// 무음 경고 조건: 루프 트랙 재생 중, All Mute 꺼짐, 모든 출력 채널 피크가 정확히 0.
pub fn silence_condition(loop_playing: bool, master_mute: bool, peaks: impl IntoIterator<Item = f32>) -> bool {
    loop_playing && !master_mute && peaks.into_iter().all(|peak| peak == 0.0)
}

/// 저장 스레드가 저장할 때마다 부른다. 경고해야 하면 누적 횟수(하트비트가 감시에 알림)를 올리고 그 값을 돌려준다.
pub fn check_silence(watch: &mut SilenceWatch, now_ms: u64) -> Option<u32> {
    if !watch.observe(now_ms, silent_now()) {
        return None;
    }
    Some(crate::core::app_signals::SILENT_WARNINGS.fetch_add(1, Ordering::Relaxed) + 1)
}

fn silent_now() -> bool {
    let playing: Vec<String> =
        GLOBAL_STATE.playing_track_ids.read().unwrap_or_else(|e| e.into_inner()).values().cloned().collect();
    let loop_playing = GLOBAL_STATE.config.read().unwrap_or_else(|e| e.into_inner()).as_ref().is_some_and(|c| {
        c.rooms.iter().flat_map(|r| r.tracks.iter()).any(|t| t.is_loop && playing.contains(&t.id))
    });
    let channels = (GLOBAL_STATE.active_device_channels.load(Ordering::Relaxed) as usize)
        .min(GLOBAL_STATE.vu_levels.len());
    silence_condition(
        loop_playing,
        GLOBAL_STATE.master_mute.load(Ordering::Relaxed),
        GLOBAL_STATE.vu_levels[..channels].iter().map(|v| f32::from_bits(v.load(Ordering::Relaxed))),
    )
}

/// 장치를 기다리는 동안 미뤄 둔 이어 가기 계획과 그때의 정지 순번. 엔진이 열리면 [resume_deferred]가 꺼내 쓴다.
static DEFERRED: Mutex<Option<(ResumePlan, u64)>> = Mutex::new(None);

fn deferred() -> std::sync::MutexGuard<'static, Option<(ResumePlan, u64)>> {
    DEFERRED.lock().unwrap_or_else(|e| e.into_inner())
}

/// 미뤄 둔 이어 가기의 (활성 방, 트랙). 첫 방 테마로 시작하는 계획이거나 없으면 None. 저장 스레드가 쓴다.
fn deferred_tracks() -> Option<(Option<String>, Vec<SavedTrack>)> {
    match deferred().as_ref() {
        Some((ResumePlan::Resume { active_room_id, tracks }, _)) => Some((active_room_id.clone(), tracks.clone())),
        _ => None,
    }
}

/// 시작할 때 읽어 둔 지난 상태로 이어 간다. 이어 갈 수 없으면 첫 방 테마로 시작한다. 읽어 둔 상태는 한 번만 쓴다.
/// 오디오 장치를 기다리거나 실패한 동안이면 지금 틀지 않고 계획을 들고 있다가 엔진이 열리면 튼다([resume_deferred]).
/// 예전에는 재생이 거절돼 이어 가기가 사라지고 공연 상태 파일도 빈 목록으로 덮여, 장치가 돌아와도 아무것도 나지 않았다
/// (2026-11-16 ④ 실기 F2: 인터페이스를 끈 채 로그인 자동 실행). 사용자 결정(2026-11-16): 장치가 오면 저장된 위치부터 —
/// 기다린 시간은 건너뛰지 않는다(로그인 이어 가기와 같다).
pub fn resume(now_ms: u64) -> Result<(), AtmosError> {
    // 이어 갈 계획을 정하는 순간의 정지 순번. 그 뒤 준비하는 사이 정지가 들어오면 그룹을 시작하지 않는다.
    let stop_epoch = crate::core::group_start::current_epoch();
    let previous = PREVIOUS.lock().unwrap_or_else(|e| e.into_inner()).take();
    let plan = {
        let config = GLOBAL_STATE.config.read().unwrap_or_else(|e| e.into_inner());
        decide_resume(previous.as_ref(), now_ms, config.as_ref())
    };
    // 설정 잠금을 놓은 뒤 거른다(파일 길이를 읽으며 설정을 다시 본다).
    let plan = drop_short_tails(plan);
    if !crate::core::engine_keeper::accepts_play() {
        let what = match &plan {
            ResumePlan::Resume { tracks, .. } => format!("트랙 {}개를 멈춘 위치부터", tracks.len()),
            ResumePlan::ThemeStart(reason) => format!("첫 방 테마로 시작 — {reason}"),
        };
        GLOBAL_STATE.log(format!("공연 이어 가기: 오디오 장치를 기다리는 중이라 장치가 열리면 시작한다({what})"));
        *deferred() = Some((plan, stop_epoch));
        // 들고 있는 사이 유지자가 이미 엔진을 열고 resume_deferred를 지나갔으면 여기서 한다(먼저 꺼낸 쪽만 튼다).
        if crate::core::engine_keeper::accepts_play() {
            resume_deferred();
        }
        return Ok(());
    }
    run_plan(plan, stop_epoch)
}

/// 이어 갈 계획에서 남은 길이(파일 길이 − 저장된 위치)가 10초 이하인 비반복 트랙을 빼고 앱 로그에 남긴다(안정성 ⑦,
/// `restart_resume::keep_for_resume`). 이 길은 꺼져 있던 시간을 건너뛰지 않으므로 저장된 위치로 잰다.
/// 모두 빠지면 트랙이 빈 계획이다 — 아무것도 틀지 않고 첫 방 테마로도 시작하지 않는다(사용자 결정 2026-11-16).
fn drop_short_tails(plan: ResumePlan) -> ResumePlan {
    let ResumePlan::Resume { active_room_id, tracks } = plan else {
        return plan;
    };
    let tracks = tracks
        .into_iter()
        .filter(|t| {
            let info = crate::core::restart_resume::track_length_info(&t.track_id);
            let is_loop = info.is_some_and(|(is_loop, _)| is_loop);
            let length = info.map(|(_, length)| length);
            let keep = crate::core::restart_resume::keep_for_resume(is_loop, length, t.seconds);
            if !keep {
                GLOBAL_STATE.log(format!(
                    "공연 이어 가기: {} 남은 길이 {:.1}초라 이어 가지 않는다(10초 이하 비반복)",
                    t.track_id,
                    length.map(|l| (l - t.seconds).max(0.0)).unwrap_or(0.0)
                ));
            }
            keep
        })
        .collect();
    ResumePlan::Resume { active_room_id, tracks }
}

/// 엔진이 (다시) 열린 뒤 엔진 유지자가 부른다. 장치를 기다리는 동안 미뤄 둔 이어 가기가 있으면 지금 한다.
/// 미룬 뒤 사용자 정지(전체·트랙·방 정지, 엔진을 직접 정지·시작)가 있었으면 버린다(정지 순번).
pub fn resume_deferred() {
    let Some((plan, epoch)) = deferred().take() else {
        return;
    };
    if crate::core::group_start::current_epoch() != epoch {
        GLOBAL_STATE.log("공연 이어 가기: 장치를 기다리는 사이 정지가 들어와 미뤄 둔 이어 가기를 버린다".to_string());
        return;
    }
    if !crate::core::engine_keeper::accepts_play() {
        *deferred() = Some((plan, epoch));
        return;
    }
    GLOBAL_STATE.log("공연 이어 가기: 오디오 장치가 열려 미뤄 둔 이어 가기를 시작한다".to_string());
    if let Err(e) = run_plan(plan, epoch) {
        GLOBAL_STATE.log(format!("공연 이어 가기 실패: {}", e.message));
    }
}

fn run_plan(plan: ResumePlan, stop_epoch: u64) -> Result<(), AtmosError> {
    match plan {
        ResumePlan::Resume { active_room_id, tracks } if tracks.is_empty() => {
            // 이어 갈 트랙이 모두 곧 끝날 것이었다(안정성 ⑦). 활성 방은 되살려 다음 OSC 큐가 그 방으로 가게 한다.
            GLOBAL_STATE.log(format!(
                "공연 이어 가기: 활성 방 {active_room_id:?}, 이어 갈 트랙이 모두 10초 이하로 남은 비반복이라 아무것도 틀지 않는다"
            ));
            GLOBAL_STATE.set_active_room(active_room_id);
            crate::osc::reply::RESUMED_TRACKS.store(0, Ordering::Relaxed);
            Ok(())
        }
        ResumePlan::Resume { active_room_id, tracks } => {
            GLOBAL_STATE.log(format!(
                "공연 이어 가기: 활성 방 {active_room_id:?}, 트랙 {}개를 멈춘 위치부터 다시 튼다",
                tracks.len()
            ));
            GLOBAL_STATE.set_active_room(active_room_id);
            // 한 그룹으로 같은 샘플에 함께 시작하고, 길이가 같은 트랙은 가장 앞선 위치로 맞춘다(그룹 시작 설계 3.5).
            // 예전에는 하나씩 틀어 위치가 뒤일수록 트랙끼리 어긋났다(진단 T264: 290초에서 최대 103ms).
            let mut plan: Vec<(String, String, f64)> =
                tracks.into_iter().map(|t| (t.room_id, t.track_id, t.seconds)).collect();
            crate::core::group_start::align_plan(&mut plan);
            let outcome = crate::core::group_start::play_group_outcome(&plan, stop_epoch);
            for (track_id, e) in &outcome.failed {
                GLOBAL_STATE.log(format!("공연 이어 가기: {track_id} 재생 실패: {}", e.message));
            }
            // OSC 시작 신호의 "이어 재생한 트랙 수"(osc::reply, 규격 §3.2). 준비하는 사이 정지가 들어와 그룹을
            // 시작하지 않았으면 이어 간 것이 없으므로 0이다.
            let resumed = if outcome.started { plan.len().saturating_sub(outcome.failed.len()) } else { 0 };
            crate::osc::reply::RESUMED_TRACKS.store(resumed as u32, Ordering::Relaxed);
            Ok(())
        }
        ResumePlan::ThemeStart(reason) => {
            GLOBAL_STATE.log(format!("공연 이어 가기 대신 첫 방 테마로 시작한다: {reason}"));
            crate::api::show::api_theme_start()
        }
    }
}
