//! 비상 전환(기본 장치로 임시 전환) 뒤 엔진이 스스로 다시 시작할 때 어느 장치로 열지 정한다.
//!
//! 장치가 30초 넘게 사라지면 엔진은 기본 장치로 비상 전환하고 출력에 −40dB를 건다(`is_failover_mode`). 이 표시는
//! 장치 이름을 지정해 열어야만 풀린다. 그런데 비상 엔진은 장치 이름 없이 열려 있어서, 원래 장치가 돌아와 엔진이 다시
//! 시작해도 이름 없이 다시 열렸고 감쇠가 남아 사실상 무음이었다(실기 2026-10-07, Scarlett 6i6). 그래서 설정에서 고른
//! 장치를 기억해 두고, 비상 상태에서 다시 시작할 때 그 장치가 장치 목록에 있으면 이름을 지정해 연다.

use std::sync::RwLock;

/// 설정에서 고른 출력 장치(None이면 기본 장치). 공개 시작 API(`api_init_audio_system`)만 바꾼다.
/// 비상 전환의 내부 재기동(장치 이름 없이 다시 열기)은 이 값을 바꾸지 않는다.
static REQUESTED_DEVICE: RwLock<Option<String>> = RwLock::new(None);

/// 설정에서 고른 장치를 기억한다.
pub fn remember_requested_device(device_name: Option<String>) {
    *REQUESTED_DEVICE.write().unwrap_or_else(|e| e.into_inner()) = device_name;
}

/// 마지막으로 기억한 설정의 장치.
pub fn requested_device() -> Option<String> {
    REQUESTED_DEVICE.read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// 엔진의 장치 검색(`AudioEngine::start`)과 같은 규칙으로 호스트 표시·널 문자·앞뒤 공백을 뺀 이름.
fn bare_name(name: &str) -> String {
    name.replace("[ASIO] ", "")
        .replace("[WASAPI] ", "")
        .replace("[CoreAudio] ", "")
        .replace('\0', "")
        .trim()
        .to_string()
}

/// 자기 재시작(워치독·장치 유실·장치 목록 변화) 때 열 장치. None이면 기본 장치다.
/// - 비상 상태가 아니면 이번 엔진을 연 장치 그대로다. 그 장치가 없으면 엔진이 30초 동안 찾다가 비상 전환한다.
/// - 비상 상태에서 설정의 장치가 지금 `available`에 있으면 그 장치를 설정의 이름 그대로 돌려준다.
///   이름을 지정해 열어야 비상 감쇠가 풀린다.
/// - 아직 없으면 이번 엔진의 장치(비상 엔진은 None, 기본 장치)에 머문다. 없는 장치를 지정하면 30초 동안 무음이다.
pub fn device_for_self_restart(
    current: Option<&str>,
    failover: bool,
    requested: Option<&str>,
    available: &[String],
) -> Option<String> {
    if failover {
        if let Some(requested) = requested {
            let want = bare_name(requested);
            if !want.is_empty() && available.iter().any(|name| bare_name(name) == want) {
                return Some(requested.to_string());
            }
        }
    }
    current.map(str::to_string)
}
