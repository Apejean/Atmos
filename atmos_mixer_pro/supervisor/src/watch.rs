//! 앱 하나를 지켜보며 멈춤을 판단한다. 하트비트 값이 바뀐 시각을 감시 프로그램의 단조 시계로 잰다 —
//! 파일의 시각 값을 벽시계와 비교하지 않으므로 시계 조정(NTP, 시간대)에 영향받지 않는다.
use crate::heartbeat::Heartbeat;
use std::time::{Duration, Instant};

/// 확인 간격이 이보다 벌어지면(절전에서 깨어남 등) 멈춤 타이머를 다시 잰다.
pub const SLEEP_GAP: Duration = Duration::from_secs(5);
/// 장치 대기·실패인데 엔진 유지자의 진행 카운터가 이만큼 그대로면 유지 스레드 멈춤이다(안정성 ⑥, 설계 5.3.1).
/// 카운터는 5초마다 늘고, 열기 시도 중에는 최대 약 20초(재시도 찾기 5초 + 시작 대기 여유 15초) 멈춘다.
pub const KEEPER_STALL: Duration = Duration::from_secs(90);
/// 장치는 보이는데 열기 실패(`Failed(open_failed)`)가 이만큼 이어지면 다시 띄운다(앱 안에서 30초마다 4번 더 시도한 뒤).
pub const OPEN_FAILED_AFTER: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Ok,
    /// 첫 하트비트가 기한 안에 오지 않았다.
    StartupHang,
    /// UI 하트비트(ui_ms)가 멈췄다.
    UiHang,
    /// 엔진이 켜져 있는데 오디오 콜백 시각이 멈췄다.
    AudioStall,
    /// 엔진 시작 호출이 돌아오지 않았다(`Failed(start_hung)`, 드라이버 멈춤). 앱 안에서는 회복할 수 없다.
    EngineStartHung,
    /// 장치 대기·실패인데 유지자 진행 카운터가 멈췄다(유지 스레드 멈춤).
    KeeperStalled,
    /// 장치는 보이는데 열기 실패가 오래 이어진다(`Failed(open_failed)`).
    EngineOpenFailed,
}

impl Verdict {
    /// 엔진 상태값으로 내린 판단인가(오디오 멈춤과 따로 센다).
    pub fn is_engine(self) -> bool {
        matches!(self, Verdict::EngineStartHung | Verdict::KeeperStalled | Verdict::EngineOpenFailed)
    }
}

/// 엔진 상태와 실패 원인.
pub type EngineKey = (String, String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub verdict: Verdict,
    /// 이번에 오디오 콜백 시각이 바뀌었다(콜백이 돈다).
    pub audio_progressed: bool,
    /// 확인 간격이 벌어져 타이머를 다시 쟀다.
    pub gap: bool,
    /// 하트비트의 엔진 상태가 `Running`이다.
    pub engine_running: bool,
    /// 엔진 상태(또는 원인)가 바뀌었다: (전, 지금). 처음 본 상태면 전은 None.
    pub engine_change: Option<(Option<EngineKey>, EngineKey)>,
}

pub struct Watch {
    first_heartbeat: Duration,
    hang: Duration,
    audio_stall: Duration,
    keeper_stall: Duration,
    open_failed_after: Duration,
    started: Instant,
    last_tick: Instant,
    /// 마지막으로 본 ui_ms와 그 값으로 바뀐 시각.
    ui: Option<(u64, Instant)>,
    /// 마지막으로 본 audio_cb_ms와 그 값으로 바뀐 시각. 엔진이 꺼져 있으면 None(판단하지 않음).
    audio: Option<(u64, Instant)>,
    /// 마지막으로 본 엔진 상태와 원인.
    engine: Option<EngineKey>,
    /// 대기·실패 중 마지막으로 본 진행 카운터와 그 값으로 바뀐 시각. 그 밖의 상태면 None.
    progress: Option<(u64, Instant)>,
    /// `Failed(open_failed)`를 처음 본 시각. 다른 상태가 되면 None.
    open_failed_since: Option<Instant>,
}

impl Watch {
    /// [first_heartbeat]는 첫 하트비트 기한이다(새로 띄운 앱 90초, 넘겨받은 앱은 멈춤 판단과 같은 30초).
    /// 엔진 상태 판단 시간은 기본값([KEEPER_STALL]·[OPEN_FAILED_AFTER])이고 [Watch::with_engine_limits]로 바꾼다.
    pub fn new(now: Instant, first_heartbeat: Duration, hang: Duration, audio_stall: Duration) -> Self {
        Self {
            first_heartbeat,
            hang,
            audio_stall,
            keeper_stall: KEEPER_STALL,
            open_failed_after: OPEN_FAILED_AFTER,
            started: now,
            last_tick: now,
            ui: None,
            audio: None,
            engine: None,
            progress: None,
            open_failed_since: None,
        }
    }

    /// 엔진 상태 판단 시간(유지자 멈춤, 열기 실패)을 정한다. 테스트는 짧게 준다.
    pub fn with_engine_limits(mut self, keeper_stall: Duration, open_failed_after: Duration) -> Self {
        self.keeper_stall = keeper_stall;
        self.open_failed_after = open_failed_after;
        self
    }

    /// 확인할 때마다 부른다. [hb]는 이 앱(PID가 같은)의 하트비트다.
    pub fn observe(&mut self, now: Instant, hb: Option<&Heartbeat>) -> Observation {
        let gap = now.saturating_duration_since(self.last_tick) > SLEEP_GAP;
        self.last_tick = now;
        if gap {
            self.started = now;
            if let Some(ui) = self.ui.as_mut() {
                ui.1 = now;
            }
            if let Some(audio) = self.audio.as_mut() {
                audio.1 = now;
            }
            if let Some(progress) = self.progress.as_mut() {
                progress.1 = now;
            }
            if self.open_failed_since.is_some() {
                self.open_failed_since = Some(now);
            }
        }
        let mut audio_progressed = false;
        let mut engine_change = None;
        if let Some(hb) = hb {
            if self.ui.map(|(value, _)| value) != Some(hb.ui_ms) {
                self.ui = Some((hb.ui_ms, now));
            }
            if !hb.engine_active {
                self.audio = None;
            } else {
                match self.audio {
                    Some((value, _)) if value == hb.audio_cb_ms => {}
                    Some(_) => {
                        audio_progressed = hb.audio_cb_ms != 0;
                        self.audio = Some((hb.audio_cb_ms, now));
                    }
                    None => self.audio = Some((hb.audio_cb_ms, now)),
                }
            }
            if let Some(engine) = &hb.engine {
                let key = (engine.state.clone(), engine.cause.clone());
                if self.engine.as_ref() != Some(&key) {
                    engine_change = Some((self.engine.replace(key.clone()), key));
                }
                if matches!(engine.state.as_str(), "WaitingForDevice" | "Failed") {
                    if self.progress.map(|(value, _)| value) != Some(engine.progress) {
                        self.progress = Some((engine.progress, now));
                    }
                } else {
                    self.progress = None;
                }
                if engine.state == "Failed" && engine.cause == "open_failed" {
                    self.open_failed_since.get_or_insert(now);
                } else {
                    self.open_failed_since = None;
                }
            }
        }
        let engine_running = self.engine.as_ref().is_some_and(|(state, _)| state == "Running");
        let start_hung =
            self.engine.as_ref().is_some_and(|(state, cause)| state == "Failed" && cause == "start_hung");
        let since = |t: Instant| now.saturating_duration_since(t);
        let verdict = match self.ui {
            None if since(self.started) > self.first_heartbeat => Verdict::StartupHang,
            Some((_, changed)) if since(changed) > self.hang => Verdict::UiHang,
            _ if start_hung => Verdict::EngineStartHung,
            _ if self.progress.is_some_and(|(_, changed)| since(changed) > self.keeper_stall) => {
                Verdict::KeeperStalled
            }
            _ if self.open_failed_since.is_some_and(|t| since(t) > self.open_failed_after) => {
                Verdict::EngineOpenFailed
            }
            _ => match self.audio {
                Some((_, changed)) if since(changed) > self.audio_stall => Verdict::AudioStall,
                _ => Verdict::Ok,
            },
        };
        Observation { verdict, audio_progressed, gap, engine_running, engine_change }
    }
}
