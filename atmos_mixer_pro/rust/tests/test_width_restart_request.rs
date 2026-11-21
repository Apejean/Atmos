//! 출력 설정 저장 한 번에 엔진이 두 번 다시 시작하던 것(2026-11-21 1.3.14 설치본 확인 12:28·12:31).
//! 저장하면 설정이 여러 번 들어오고, 옛 엔진을 내리는 사이(처리 폭 값이 아직 옛 값) 들어온 설정이 재시작 요청을 다시
//! 남겨 새 엔진이 뜨자마자 한 번 더 재시작했다. 이제 엔진 관리 스레드가 요청을 꺼낼 때 지금 연 믹서 폭과 원하는 폭을
//! 다시 비교해 같으면 건너뛴다. 전역 상태를 쓰므로 시험은 하나다.
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, ChannelSetting};
use rust_lib_atmos_mixer_pro::core::processing_channels::{take_width_restart_request, width_change};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;

fn on() -> ChannelSetting {
    ChannelSetting {
        enabled: true,
        custom_name: String::new(),
        delay_ms: 0.0,
        eq_bands: Vec::new(),
        position: None,
        phase_invert: false,
        gain_db: 0.0,
    }
}

#[test]
fn 이미_원하는_폭으로_열린_엔진은_남은_요청으로_다시_시작하지_않는다() {
    // 출력 설정에서 모노 CH21을 켰다 → 처리 폭 21.
    let mut config = AppConfig::default();
    config.mono_configs.insert(21, on());
    *GLOBAL_STATE.config.write().unwrap() = Some(config.clone());
    GLOBAL_STATE.active_device_channels.store(94, Ordering::SeqCst);

    // 옛 엔진(믹서 폭 16)이 돌고 있다: 폭이 다르다.
    GLOBAL_STATE.processing_channels.store(16, Ordering::SeqCst);
    assert_eq!(width_change(&config), Some((16, 21)), "옛 폭 16, 원하는 폭 21");
    GLOBAL_STATE.processing_width_restart.store(true, Ordering::SeqCst);
    assert!(take_width_restart_request(), "폭이 다르면 다시 시작한다");
    assert!(!GLOBAL_STATE.processing_width_restart.load(Ordering::SeqCst), "요청은 꺼내면 지운다");

    // 옛 엔진을 내리는 사이 들어온 같은 설정이 요청을 다시 남겼고, 새 엔진은 이미 21로 열렸다.
    GLOBAL_STATE.processing_width_restart.store(true, Ordering::SeqCst);
    GLOBAL_STATE.processing_channels.store(21, Ordering::SeqCst);
    assert_eq!(width_change(&config), None, "이미 원하는 폭");
    assert!(!take_width_restart_request(), "남은 요청으로 한 번 더 다시 시작했다(저장 한 번에 두 번 끊김)");
    assert!(!GLOBAL_STATE.processing_width_restart.load(Ordering::SeqCst), "건너뛴 요청도 지운다");

    // 요청이 없으면 폭이 달라도 다시 시작하지 않는다(요청은 설정·스피커 배치 변경이 남긴다).
    GLOBAL_STATE.processing_channels.store(16, Ordering::SeqCst);
    assert!(!take_width_restart_request(), "요청 없음");

    // 엔진이 없거나 장치 채널 수를 모르면 비교하지 않는다.
    GLOBAL_STATE.processing_channels.store(0, Ordering::SeqCst);
    assert_eq!(width_change(&config), None, "엔진 없음");
}
