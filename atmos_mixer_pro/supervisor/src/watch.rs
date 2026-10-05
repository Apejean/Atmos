//! 앱 하나를 지켜보며 멈춤을 판단한다. 하트비트 값이 바뀐 시각을 감시 프로그램의 단조 시계로 잰다 —
//! 파일의 시각 값을 벽시계와 비교하지 않으므로 시계 조정(NTP, 시간대)에 영향받지 않는다.
use crate::heartbeat::Heartbeat;
use std::time::{Duration, Instant};

/// 확인 간격이 이보다 벌어지면(절전에서 깨어남 등) 멈춤 타이머를 다시 잰다.
pub const SLEEP_GAP: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Ok,
    /// 첫 하트비트가 기한 안에 오지 않았다.
    StartupHang,
    /// UI 하트비트(ui_ms)가 멈췄다.
    UiHang,
    /// 엔진이 켜져 있는데 오디오 콜백 시각이 멈췄다.
    AudioStall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Observation {
    pub verdict: Verdict,
    /// 이번에 오디오 콜백 시각이 바뀌었다(콜백이 돈다).
    pub audio_progressed: bool,
    /// 확인 간격이 벌어져 타이머를 다시 쟀다.
    pub gap: bool,
}

pub struct Watch {
    first_heartbeat: Duration,
    hang: Duration,
    audio_stall: Duration,
    started: Instant,
    last_tick: Instant,
    /// 마지막으로 본 ui_ms와 그 값으로 바뀐 시각.
    ui: Option<(u64, Instant)>,
    /// 마지막으로 본 audio_cb_ms와 그 값으로 바뀐 시각. 엔진이 꺼져 있으면 None(판단하지 않음).
    audio: Option<(u64, Instant)>,
}

impl Watch {
    /// [first_heartbeat]는 첫 하트비트 기한이다(새로 띄운 앱 90초, 넘겨받은 앱은 멈춤 판단과 같은 30초).
    pub fn new(now: Instant, first_heartbeat: Duration, hang: Duration, audio_stall: Duration) -> Self {
        Self { first_heartbeat, hang, audio_stall, started: now, last_tick: now, ui: None, audio: None }
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
        }
        let mut audio_progressed = false;
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
        }
        let since = |t: Instant| now.saturating_duration_since(t);
        let verdict = match self.ui {
            None if since(self.started) > self.first_heartbeat => Verdict::StartupHang,
            Some((_, changed)) if since(changed) > self.hang => Verdict::UiHang,
            _ => match self.audio {
                Some((_, changed)) if since(changed) > self.audio_stall => Verdict::AudioStall,
                _ => Verdict::Ok,
            },
        };
        Observation { verdict, audio_progressed, gap }
    }
}
