//! 앱이 남기는 신호 파일(앱 로그 폴더). 앱(rust/src/core/app_signals.rs)과 이름·형식이 같아야 한다.
use std::path::Path;

pub const APP_LOCK: &str = "app.lock";
pub const APP_PID: &str = "app.pid";
pub const HEARTBEAT: &str = "heartbeat";
pub const CLEAN_EXIT: &str = "clean_exit";
pub const SUPERVISOR_LOCK: &str = "supervisor.lock";

/// 앱의 엔진 유지자 상태(안정성 ③부터 하트비트에 실림). 감시는 이것으로 다시 띄울지 정한다(안정성 ⑥, 설계 5.3.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineReport {
    /// `Starting`·`Running`·`WaitingForDevice`·`Failed`·`StoppedByUser`
    pub state: String,
    /// 진행 카운터. 대기·실패 중에도 5초마다 는다(유지자가 살아 있다).
    pub progress: u64,
    /// 실패 원인(`start_hung`·`open_failed`·`unavailable`). 실패가 아니거나 ③만 있는 앱이면 빈 값.
    pub cause: String,
}

/// 앱이 2초마다 쓰는 하트비트(`키=값` 줄).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heartbeat {
    pub pid: u32,
    /// UI가 하트비트를 부른 시각(밀리초). 값이 바뀌는지만 본다.
    pub ui_ms: u64,
    /// 마지막 오디오 콜백 시각(밀리초). 값이 바뀌는지만 본다.
    pub audio_cb_ms: u64,
    pub engine_active: bool,
    /// 앱이 남긴 재생 중 무음 경고 누적 횟수.
    pub silent_warnings: u32,
    /// 엔진 유지자 상태. 옛 앱(③ 전)이거나 값이 깨졌으면 None — 그때는 지금 규칙으로 판단한다.
    pub engine: Option<EngineReport>,
}

impl Heartbeat {
    /// 다섯 키가 모두 있어야 읽는다(빠진 키·잘린 값은 None). 엔진 상태 세 키는 선택이다:
    /// `engine_state`·`engine_progress`가 둘 다 읽히면 [EngineReport]를 만들고, `engine_cause`가 없으면 빈 값.
    pub fn parse(text: &str) -> Option<Heartbeat> {
        let (mut pid, mut ui_ms, mut audio_cb_ms, mut engine_active, mut silent_warnings) =
            (None, None, None, None, None);
        let (mut engine_state, mut engine_progress, mut engine_cause) = (None, None, None);
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else { continue };
            let value = value.trim();
            match key.trim() {
                "pid" => pid = value.parse().ok(),
                "ui_ms" => ui_ms = value.parse().ok(),
                "audio_cb_ms" => audio_cb_ms = value.parse().ok(),
                "engine_active" => {
                    engine_active = match value {
                        "1" => Some(true),
                        "0" => Some(false),
                        _ => None,
                    }
                }
                "silent_warnings" => silent_warnings = value.parse().ok(),
                "engine_state" if !value.is_empty() => engine_state = Some(value.to_string()),
                "engine_progress" => engine_progress = value.parse().ok(),
                "engine_cause" => engine_cause = Some(value.to_string()),
                _ => {}
            }
        }
        let engine = match (engine_state, engine_progress) {
            (Some(state), Some(progress)) => {
                Some(EngineReport { state, progress, cause: engine_cause.unwrap_or_default() })
            }
            _ => None,
        };
        Some(Heartbeat {
            pid: pid?,
            ui_ms: ui_ms?,
            audio_cb_ms: audio_cb_ms?,
            engine_active: engine_active?,
            silent_warnings: silent_warnings?,
            engine,
        })
    }

    pub fn render(&self) -> String {
        let mut text = format!(
            "pid={}\nui_ms={}\naudio_cb_ms={}\nengine_active={}\nsilent_warnings={}\n",
            self.pid,
            self.ui_ms,
            self.audio_cb_ms,
            u8::from(self.engine_active),
            self.silent_warnings
        );
        if let Some(engine) = &self.engine {
            text.push_str(&format!(
                "engine_state={}\nengine_progress={}\nengine_cause={}\n",
                engine.state, engine.progress, engine.cause
            ));
        }
        text
    }
}

/// [dir]의 하트비트. 없거나 형식이 맞지 않으면 None.
pub fn read_heartbeat(dir: &Path) -> Option<Heartbeat> {
    Heartbeat::parse(&std::fs::read_to_string(dir.join(HEARTBEAT)).ok()?)
}

/// 숫자만 있는 파일(app.pid) 또는 `pid=` 한 줄(clean_exit)에서 PID를 읽는다.
pub fn read_pid(path: &Path) -> Option<u32> {
    let text = std::fs::read_to_string(path).ok()?;
    let text = text.trim();
    text.strip_prefix("pid=").unwrap_or(text).trim().parse().ok()
}

/// 임시 파일에 쓰고 이름을 바꾼다. 읽는 쪽이 반쯤 쓴 내용을 보지 않는다.
pub fn atomic_write(path: &Path, contents: &str) -> std::io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}
