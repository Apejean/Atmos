//! 루프 더킹 설정(검토 A4, 사용자 결정 2026-11-16 "더킹은 의도된 동작").
//!
//! 전시 모드가 아니면 비루프 트랙이 나는 동안 모든 방의 루프를 줄인다(audio::mixer). 이 모듈은 그 켜기·끄기와
//! 줄이는 양을 설정(`AppConfig.ducking_enabled`·`ducking_reduction_db`)에서 원자값으로 옮기고(오디오 스레드 밖),
//! 믹서가 프레임마다 쓰는 무게 계산과 화면 표시 갱신을 맡는다. 믹서는 원자값을 읽기만 한다(3법칙 1·2).
//! 기본값(켜짐, 20·log10(0.3)dB)에서는 무게가 예전 코드(0.3 상수)와 비트 단위로 같다.

use crate::common::config::AppConfig;
use crate::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Once;
use std::time::Duration;

/// 기본 줄이는 양: 예전 고정값 0.3배와 같은 20·log10(0.3).
pub const DEFAULT_REDUCTION_DB: f32 = -10.457_575;
/// 예전 고정 무게. 기본 설정에서는 계산하지 않고 이 값을 그대로 쓴다(비트 단위로 같게).
pub const DEFAULT_FLOOR: f32 = 0.3;
/// 설정할 수 있는 가장 큰 줄임.
pub const MIN_REDUCTION_DB: f32 = -40.0;
/// 램프 기울기의 폭: 예전 코드의 0.7(=1−0.3)을 그대로 쓴다. 내림은 이 폭을 150ms, 올림은 300ms에 움직인다.
/// 줄이는 양을 바꿔도 기울기는 같아서 0dB로 바꿔도 멈추지 않는다.
const RAMP_SPAN: f32 = 0.7;
/// 화면 표시를 갱신하는 감시 간격.
const INDICATOR_POLL: Duration = Duration::from_millis(100);

/// 줄이는 양(dB)을 무게(선형)로 바꾼다. −40~0dB로 자르고, 숫자가 아니거나 기본값이면 0.3을 그대로 돌려준다.
pub fn floor_gain(reduction_db: f32) -> f32 {
    if !reduction_db.is_finite() || (reduction_db - DEFAULT_REDUCTION_DB).abs() < 1e-4 {
        return DEFAULT_FLOOR;
    }
    let db = reduction_db.clamp(MIN_REDUCTION_DB, 0.0);
    10f32.powf(db / 20.0)
}

/// 설정을 엔진 원자값에 넣는다. 설정을 읽거나 저장하거나 프리셋을 불러올 때(오디오 스레드 밖) 부른다.
/// 처음 부를 때 화면 표시 감시 스레드를 띄운다.
pub fn apply_settings(config: &AppConfig) {
    GLOBAL_STATE.ducking_enabled.store(config.ducking_enabled, Ordering::Relaxed);
    GLOBAL_STATE
        .ducking_floor
        .store(floor_gain(config.ducking_reduction_db).to_bits(), Ordering::Relaxed);
    start_indicator_watch();
}

/// 프레임 하나만큼 더킹 무게를 움직인다(믹서가 프레임마다 부름, 할당·잠금 없음).
/// 더킹 중이면 `floor`로, 아니면 1.0으로 간다. 내림은 `down_frames`, 올림은 `up_frames`에 폭 0.7을 움직이는 기울기다.
/// 더킹 중에 `floor`가 올라가면(덜 줄이기) 올림 기울기로 새 `floor`까지 올라간다(한 번에 뛰지 않는다).
pub fn step_weight(weight: f32, ducking: bool, floor: f32, down_frames: usize, up_frames: usize) -> f32 {
    let mut w = weight;
    if ducking {
        if w > floor {
            w -= RAMP_SPAN / down_frames as f32;
            if w < floor {
                w = floor;
            }
        } else if w < floor {
            w += RAMP_SPAN / up_frames as f32;
            if w > floor {
                w = floor;
            }
        }
    } else {
        if w < 1.0 {
            w += RAMP_SPAN / up_frames as f32;
        }
        if w > 1.0 {
            w = 1.0;
        }
    }
    w
}

/// 믹서가 정한 더킹 상태(`ducking_engaged`)를 화면 상태(`is_ducking`)에 옮긴다. 바뀌었으면 상태 스트림으로 알리고
/// true를 돌려준다. 잠금·방송이 있어 오디오 스레드에서는 부르지 않는다(감시 스레드가 부른다).
pub fn sync_indicator() -> bool {
    let engaged = GLOBAL_STATE.ducking_engaged.load(Ordering::Relaxed);
    if GLOBAL_STATE.is_ducking.load(Ordering::Relaxed) == engaged {
        return false;
    }
    GLOBAL_STATE.set_ducking(engaged);
    true
}

fn start_indicator_watch() {
    static START: Once = Once::new();
    START.call_once(|| {
        let spawned = std::thread::Builder::new()
            .name("ducking-indicator".to_string())
            .spawn(|| loop {
                std::thread::sleep(INDICATOR_POLL);
                sync_indicator();
            });
        if spawned.is_err() {
            GLOBAL_STATE.log("더킹 표시 감시 스레드를 만들지 못했다(화면 표시만 갱신되지 않음)".to_string());
        }
    });
}
