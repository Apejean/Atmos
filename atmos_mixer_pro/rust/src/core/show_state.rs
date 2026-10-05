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

/// 시작할 때 읽어 둔 지난 상태로 이어 간다. 이어 갈 수 없으면 첫 방 테마로 시작한다. 읽어 둔 상태는 한 번만 쓴다.
pub fn resume(now_ms: u64) -> Result<(), AtmosError> {
    let previous = PREVIOUS.lock().unwrap_or_else(|e| e.into_inner()).take();
    let plan = {
        let config = GLOBAL_STATE.config.read().unwrap_or_else(|e| e.into_inner());
        decide_resume(previous.as_ref(), now_ms, config.as_ref())
    };
    match plan {
        ResumePlan::Resume { active_room_id, tracks } => {
            GLOBAL_STATE.log(format!(
                "공연 이어 가기: 활성 방 {active_room_id:?}, 트랙 {}개를 멈춘 위치부터 다시 튼다",
                tracks.len()
            ));
            GLOBAL_STATE.set_active_room(active_room_id);
            for t in tracks {
                if let Err(e) = crate::api::simple::play_track_from(t.room_id, t.track_id.clone(), t.seconds) {
                    GLOBAL_STATE.log(format!("공연 이어 가기: {} 재생 실패: {}", t.track_id, e.message));
                }
            }
            Ok(())
        }
        ResumePlan::ThemeStart(reason) => {
            GLOBAL_STATE.log(format!("공연 이어 가기 대신 첫 방 테마로 시작한다: {reason}"));
            crate::api::show::api_theme_start()
        }
    }
}
