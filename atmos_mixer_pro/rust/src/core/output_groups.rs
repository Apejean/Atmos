//! 출력 설정 = 열린 채널 하나(사용자 결정 2026-11-21 "채널 하나로 묶기",
//! 설계 docs/superpowers/specs/2026-11-21-output-config-open-channels-design.md).
//!
//! 출력 설정의 모노·스테레오·멀티 세 맵은 모두 "어느 채널을 여는가"만 뜻한다: 켠 모노 k는 CHk, 켠 스테레오 k는
//! CHk·CHk+1, 켠 멀티 k는 CHk(시작 채널 하나 — 예전에는 k부터 장치 끝까지 열었다). 채널 게이트·처리 폭·트랙이
//! 소리를 내도 되는가를 모두 이 열린 채널로 정한다. 화면(`lib/core/utils/channel_routing.dart`)도 같은 규칙이다.
//! 트랙 출력 자리가 열려 있지 않으면 화면에 "출력 설정에 없음"이 뜨고 믹서는 그 트랙을 무음으로 둔다(같은 날 앞 결정
//! "A. 안내 뜨는 트랙은 무음").
use crate::common::config::AppConfig;
use std::sync::atomic::Ordering;

/// 열린 채널(0부터, 길이 `len`). 켠 모노 k → k-1, 켠 스테레오 k → k-1·k, 켠 멀티 k → k-1. 0번 키와 `len` 밖은 뺀다.
/// 켠 것이 없으면 모두 닫힘(소리 없음).
pub fn open_channels(config: &AppConfig, len: usize) -> Vec<bool> {
    let mut open = vec![false; len];
    let mut set = |index: usize| {
        if let Some(slot) = open.get_mut(index) {
            *slot = true;
        }
    };
    for (&key, setting) in &config.mono_configs {
        if setting.enabled && key >= 1 {
            set((key - 1) as usize);
        }
    }
    for (&key, setting) in &config.stereo_configs {
        if setting.enabled && key >= 1 {
            set((key - 1) as usize);
            set(key as usize);
        }
    }
    for (&key, setting) in &config.multi_configs {
        if setting.enabled && key >= 1 {
            set((key - 1) as usize);
        }
    }
    open
}

/// 트랙 출력 자리의 종류. 파일 채널 수와 트랙 `output_stereo`로 정한다(화면과 같다).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupKind {
    Mono,
    Stereo,
    Multi,
}

/// 다채널 파일(3 이상)은 멀티 자리, 모노 파일은 모노 자리(새 트랙 기본값 `output_stereo: true`도 같은 채널의 모노로
/// 본다 — 화면은 모노 파일에 모노 자리만 내놓고 트랙 카드가 그 표시만 모노로 고친다), 스테레오 파일은 스테레오 출력이면
/// 스테레오 자리·아니면 모노 자리.
pub fn required_group(file_channels: usize, output_stereo: bool) -> GroupKind {
    match file_channels {
        0..=1 => GroupKind::Mono,
        2 if output_stereo => GroupKind::Stereo,
        2 => GroupKind::Mono,
        _ => GroupKind::Multi,
    }
}

/// 자리가 열려 있는가(순수 함수). 모노·멀티 자리 c는 c가 열림, 스테레오 자리 c는 짝 시작(0부터 짝수 — 화면의 1/2·3/4…)
/// 이고 c·c+1이 모두 열림.
pub fn route_open_in(open: impl Fn(usize) -> bool, kind: GroupKind, output_channel: usize) -> bool {
    match kind {
        GroupKind::Mono | GroupKind::Multi => open(output_channel),
        GroupKind::Stereo => output_channel.is_multiple_of(2) && open(output_channel) && open(output_channel + 1),
    }
}

/// 오디오 스레드: 이 트랙 출력 자리가 지금 열려 있는가. 채널 게이트(`GLOBAL_STATE.enabled_channels`, 원자형)만 읽는다
/// (할당·잠금 없음). 장치 채널(`out_channels`) 밖은 닫힘.
pub fn route_open(file_channels: usize, output_channel: usize, output_stereo: bool, out_channels: usize) -> bool {
    let gate = &crate::core::state::GLOBAL_STATE.enabled_channels;
    let open = |index: usize| index < out_channels && gate.get(index).is_some_and(|slot| slot.load(Ordering::Relaxed));
    route_open_in(open, required_group(file_channels, output_stereo), output_channel)
}
