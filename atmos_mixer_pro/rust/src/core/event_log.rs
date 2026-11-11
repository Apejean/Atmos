//! 사건 로그: "어떤 신호·동작 뒤에 소리가 끊겼는가"를 사후에 알 수 있게 남긴다.
//!
//! - 최근 재생·정지 요청(OSC·화면)을 작은 고리 버퍼에 기억한다.
//! - 엔진이 멈추거나 복구에 실패하면 그 버퍼와 원인을 `사건:` 줄로 로그 파일에 남긴다.
//! - 동작은 바꾸지 않는다. 실시간 오디오 콜백에서는 부르지 않는다(Mutex·할당 사용).
//!
//! 상태 스트림 값 계약(Rust → Dart, `api_create_stream_status_stream`):
//! `Running` · `Failover` · `Stopped` · `HotReloading` · `WatchdogActive` · `AutoGuardActive`
//! · `EngineDown:<짧은 원인>` (복구 실패로 엔진이 내려간 상태, 배너를 계속 보인다).

use std::collections::VecDeque;
use std::sync::Mutex;

/// 기억하는 최근 요청 수.
pub const RECENT_CAPACITY: usize = 32;
/// 엔진 정지 사건에 함께 남기는 최근 요청 수.
pub const DUMP_COUNT: usize = 8;
/// 상태 스트림에서 복구 실패를 알리는 접두사.
pub const ENGINE_DOWN_PREFIX: &str = "EngineDown:";
/// 자기 재시작과 기본 장치 전환이 모두 실패한 원인. 사건 로그와 상태 스트림(배너·OSC)이 같은 값을 쓴다.
pub const CAUSE_RESTART_FAILED: &str = "restart_failed";

struct Entry {
    at: String,
    text: String,
}

static RECENT: Mutex<VecDeque<Entry>> = Mutex::new(VecDeque::new());

fn now_text() -> String {
    chrono::Local::now().format("%H:%M:%S%.3f").to_string()
}

/// 값에 공백·줄바꿈이 있으면 한 줄 key=value 형식이 깨지므로 정리한다.
fn clean(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .collect()
}

/// 사건 한 줄의 본문(`사건: 종류 key=value ...`). 시각은 로그 파일이 붙인다.
pub fn format_event(kind: &str, fields: &[(&str, &str)]) -> String {
    let mut line = format!("사건: {}", clean(kind));
    for (key, value) in fields {
        line.push(' ');
        line.push_str(&clean(key));
        line.push('=');
        line.push_str(&clean(value));
    }
    line
}

/// 재생·정지 같은 요청을 기억한다. `kind` 예: `play`, `stop`, `stop_all`, `osc_unregistered`.
pub fn record_request(kind: &str, target: &str) {
    let mut guard = RECENT.lock().unwrap_or_else(|e| e.into_inner());
    if guard.len() >= RECENT_CAPACITY {
        guard.pop_front();
    }
    guard.push_back(Entry {
        at: now_text(),
        text: format!("{}:{}", clean(kind), clean(target)),
    });
}

/// 가장 최근 `n`개를 오래된 순서로 돌려준다(`시각 종류:대상`).
pub fn recent_requests(n: usize) -> Vec<String> {
    let guard = RECENT.lock().unwrap_or_else(|e| e.into_inner());
    let skip = guard.len().saturating_sub(n);
    guard
        .iter()
        .skip(skip)
        .map(|e| format!("{} {}", e.at, e.text))
        .collect()
}

/// 사건을 로그 파일(과 콘솔)에 남긴다.
pub fn log_event(kind: &str, fields: &[(&str, &str)]) {
    crate::core::state::GLOBAL_STATE.log(format_event(kind, fields));
}

/// 엔진이 내려갈 때 원인과 직전 요청을 남긴다.
pub fn log_engine_stop(reason: &str, will_retry: bool) {
    let recent = recent_requests(DUMP_COUNT).join(" | ");
    let retry = if will_retry { "yes" } else { "no" };
    log_event(
        "engine_stop",
        &[
            ("reason", reason),
            ("retry", retry),
            ("recent", if recent.is_empty() { "none" } else { &recent }),
        ],
    );
}

/// 복구 실패 상태 문자열(`EngineDown:<원인>`).
pub fn engine_down_status(cause: &str) -> String {
    format!("{}{}", ENGINE_DOWN_PREFIX, clean(cause))
}

/// 상태 문자열이 `EngineDown:<원인>`이면 원인을 돌려준다.
pub fn parse_engine_down(status: &str) -> Option<&str> {
    status.strip_prefix(ENGINE_DOWN_PREFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_has_prefix_and_key_value() {
        let line = format_event("engine_stop", &[("reason", "watchdog gap"), ("retry", "no")]);
        assert_eq!(line, "사건: engine_stop reason=watchdog_gap retry=no");
        assert!(!line.contains('\n'));
    }

    #[test]
    fn ring_buffer_keeps_only_capacity_and_order() {
        for i in 0..(RECENT_CAPACITY + 5) {
            record_request("play", &format!("t{i}"));
        }
        let all = recent_requests(RECENT_CAPACITY + 10);
        assert_eq!(all.len(), RECENT_CAPACITY);
        assert!(all.last().unwrap().ends_with(&format!("play:t{}", RECENT_CAPACITY + 4)));
        let last3 = recent_requests(3);
        assert_eq!(last3.len(), 3);
        assert!(last3[0].ends_with(&format!("play:t{}", RECENT_CAPACITY + 2)));
    }

    #[test]
    fn engine_down_status_round_trip() {
        let s = engine_down_status("device not found");
        assert_eq!(s, "EngineDown:device_not_found");
        assert_eq!(parse_engine_down(&s), Some("device_not_found"));
        assert_eq!(parse_engine_down("Running"), None);
    }

    #[test]
    fn restart_failed_cause_is_fixed_contract() {
        // Dart 배너와 OSC 규격(status/engine)이 이 문자열을 그대로 쓴다.
        assert_eq!(CAUSE_RESTART_FAILED, "restart_failed");
        assert_eq!(engine_down_status(CAUSE_RESTART_FAILED), "EngineDown:restart_failed");
    }
}
