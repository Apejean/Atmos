//! 믹서가 처리할 출력 채널 수(처리 폭).
//!
//! 장치는 지원하는 최대 채널로 연다(WASAPI·CoreAudio는 채널 수를 줄여 열면 실패할 수 있다). 대신
//! 믹서는 출력 설정에서 켠 가장 높은 채널까지만 처리한다. 채널마다 크로스오버·채널 DSP·리미터가
//! 샘플 단위로 돌아 처리 시간이 채널 수에 비례하기 때문이다. 2026-10-08 Windows에서 RME MADIface USB
//! (출력 94채널)를 멀티 그룹 규칙 때문에 전부 처리해, 48kHz·1024프레임 예산(21.3ms)의 77%를 믹서가
//! 쓰고 최악에는 넘겨 소리가 끊기고 느려졌다(출력 설정에서는 CH40까지만 켜 둔 상태).
use crate::common::config::AppConfig;

/// 처리할 채널 수 = 가장 높은 열린 채널(1부터 센 번호, `core::output_groups::open_channels`). 결과는 `hw_channels`를
/// 넘지 않는다. 열린 채널이 없으면 0 — 어느 채널로도 내보내지 않는다(사용자 결정 2026-11-21 "아무것도 안 켜면 소리
/// 없음"). 믹서 폭은 [`mixer_width`]로 최소 16이다. 멀티 k는 시작 채널 하나만 연다(2026-11-21 "채널 하나로 묶기") —
/// 예전의 "멀티가 가장 높으면 파일 길이를 몰라 장치 끝까지" 규칙은 없어졌다. 장치 채널 수를 넘는 번호(예전 화면이 남긴
/// 255 등)는 세지 않는다.
pub fn processing_channel_count(config: &AppConfig, hw_channels: usize) -> usize {
    crate::core::output_groups::open_channels(config, hw_channels)
        .iter()
        .rposition(|&open| open)
        .map_or(0, |index| index + 1)
}

/// 믹서 폭. 2채널 장치도 믹서는 16채널 확장 버스로 돌아 왔으므로(엔진의 기존 동작) 최소 16이다.
pub fn mixer_width(processing_channels: usize) -> usize {
    16.max(processing_channels)
}

/// 지금 설정이 정하는 믹서 폭이 엔진이 연 믹서 폭과 다르면 `Some((연 폭, 원하는 폭))`. 엔진이 없거나 장치 채널 수를
/// 모르면 None.
pub fn width_change(config: &AppConfig) -> Option<(usize, usize)> {
    use std::sync::atomic::Ordering;
    let state = &crate::core::state::GLOBAL_STATE;
    let hw_channels = state.active_device_channels.load(Ordering::SeqCst) as usize;
    let current = state.processing_channels.load(Ordering::SeqCst) as usize;
    if hw_channels == 0 || current == 0 {
        return None;
    }
    let wanted = mixer_width(processing_channel_count(config, hw_channels));
    (wanted != current).then_some((current, wanted))
}

/// 엔진 관리 스레드가 부른다: 처리 폭 재시작 요청(`GLOBAL_STATE.processing_width_restart`)을 꺼내 지우고, 지금 연 믹서 폭이
/// 아직도 원하는 폭과 다를 때만 true. 출력 설정을 한 번 저장하면 설정이 여러 번 들어오는데, 옛 엔진을 내리는 사이(처리 폭
/// 값이 아직 옛 값) 들어온 설정이 요청을 다시 남겨 새 엔진이 뜨자마자 한 번 더 재시작했다(2026-11-21 1.3.14 확인, 짧은 끊김
/// 두 번). 새 엔진은 시작할 때 지금 설정으로 폭을 정하므로 그 요청은 이미 반영돼 있다 — 건너뛴다.
pub fn take_width_restart_request() -> bool {
    use std::sync::atomic::Ordering;
    let state = &crate::core::state::GLOBAL_STATE;
    if !state.processing_width_restart.swap(false, Ordering::AcqRel) {
        return false;
    }
    let config = state.config.read().unwrap_or_else(|e| e.into_inner()).clone();
    config.as_ref().and_then(width_change).is_some()
}
