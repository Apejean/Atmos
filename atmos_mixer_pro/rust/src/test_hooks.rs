//! 통합 테스트 전용 결함 주입. 장치 오류 경로가 세우는 신호를 세워 워치독과 같은 자기 재시작 경로를 탄다.
//! FRB 공개 API(`api/`)가 아니고 디버그 빌드에만 있다(lib.rs의 `#[cfg(debug_assertions)]`).
//! cargokit은 Flutter profile·release 빌드를 `--release`로 빌드하므로 그 바이너리에는 이 심볼이 없다.
use std::sync::atomic::Ordering;

#[no_mangle]
pub extern "C" fn atmos_test_request_engine_recovery() {
    crate::core::state::GLOBAL_STATE
        .device_needs_reset
        .store(true, Ordering::Release);
}
