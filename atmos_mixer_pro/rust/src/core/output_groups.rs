//! 출력 설정 = 줄마다 따로(Ableton처럼, 사용자 결정 2026-11-22 "이전처럼 에이블톤처럼 따로 하자",
//! 설계 docs/superpowers/specs/2026-11-22-output-config-separate-rows-design.md).
//!
//! 출력 설정의 모노·스테레오·멀티 줄은 서로 따라 바뀌지 않는다. 트랙은 자기 출력 종류의 줄이 켜져 있어야 소리를 낸다:
//! 모노 자리 CHk → 모노 줄 k, 스테레오 자리 CHk/k+1 → 스테레오 줄 k, 다채널 파일 CHk 시작 → 멀티 줄 k. 맞는 줄이 꺼져
//! 있으면 화면에 "출력 설정에 없음"이 뜨고 믹서는 그 트랙을 무음으로 둔다(2026-11-21 "A. 안내 뜨는 트랙은 무음").
//! 채널 게이트(어느 장치 채널로 내보내는가)는 켠 줄이 닿는 채널을 모두 합친 것이다: 모노 k → CHk, 스테레오 k → CHk·
//! CHk+1, 멀티 k → CHk부터 그 자리를 쓰는 다채널 트랙의 파일 채널 수만큼(트랙이 없으면 CHk 하나). 2026-11-21~22(1.3.15·
//! 1.3.16)에는 세 줄을 "열린 채널" 하나로 묶었고, 멀티 k는 CHk 하나만 열었다.
use crate::common::config::{AppConfig, TrackConfig};
use std::sync::atomic::Ordering;

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

/// 설정에서 그 종류의 줄이 켜져 있는가. `start`는 0부터 센 시작 채널(맵 키는 `start + 1`).
pub fn row_enabled(config: &AppConfig, kind: GroupKind, start: usize) -> bool {
    let map = match kind {
        GroupKind::Mono => &config.mono_configs,
        GroupKind::Stereo => &config.stereo_configs,
        GroupKind::Multi => &config.multi_configs,
    };
    u32::try_from(start + 1)
        .ok()
        .and_then(|key| map.get(&key))
        .is_some_and(|setting| setting.enabled)
}

/// 자리가 열려 있는가(순수 함수). 그 종류의 줄 `start`가 켜져 있고 장치 안이어야 한다. 스테레오 자리는 짝 시작(0부터
/// 짝수 — 화면의 1/2·3/4…)이고 짝 채널도 장치 안이어야 한다.
pub fn route_open_in(
    row_on: impl Fn(GroupKind, usize) -> bool,
    kind: GroupKind,
    output_channel: usize,
    out_channels: usize,
) -> bool {
    if output_channel >= out_channels || !row_on(kind, output_channel) {
        return false;
    }
    match kind {
        GroupKind::Mono | GroupKind::Multi => true,
        GroupKind::Stereo => output_channel.is_multiple_of(2) && output_channel + 1 < out_channels,
    }
}

/// 오디오 스레드: 이 트랙 출력 자리가 지금 열려 있는가. 종류별 줄 표(`GLOBAL_STATE.{mono,stereo,multi}_rows_open`,
/// 원자형)만 읽는다(할당·잠금 없음). 장치 채널(`out_channels`) 밖은 닫힘.
pub fn route_open(file_channels: usize, output_channel: usize, output_stereo: bool, out_channels: usize) -> bool {
    let state = &crate::core::state::GLOBAL_STATE;
    let row_on = |kind: GroupKind, start: usize| {
        let table = match kind {
            GroupKind::Mono => &state.mono_rows_open,
            GroupKind::Stereo => &state.stereo_rows_open,
            GroupKind::Multi => &state.multi_rows_open,
        };
        table.get(start).is_some_and(|slot| slot.load(Ordering::Relaxed))
    };
    route_open_in(row_on, required_group(file_channels, output_stereo), output_channel, out_channels)
}

/// 설정의 세 맵을 종류별 줄 표에 옮긴다(오디오 스레드 밖 — 설정을 읽거나 바꿀 때). 표 길이 밖 키와 0번 키는 뺀다.
pub fn apply_row_tables(config: &AppConfig) {
    let state = &crate::core::state::GLOBAL_STATE;
    for (kind, table) in [
        (GroupKind::Mono, &state.mono_rows_open),
        (GroupKind::Stereo, &state.stereo_rows_open),
        (GroupKind::Multi, &state.multi_rows_open),
    ] {
        for (start, slot) in table.iter().enumerate() {
            slot.store(row_enabled(config, kind, start), Ordering::Relaxed);
        }
    }
}

/// 다채널 트랙이 켠 멀티 줄에서 닿는 채널 수. 파일 채널이 3 이상이고 스테레오 출력이 켜진 트랙만 파일 채널 수만큼
/// 퍼진다(스테레오 출력을 끄면 믹서가 모노로 합쳐 시작 채널 하나로 낸다 — audio/mixer.rs). 아니면 None.
fn multi_span(track: &TrackConfig, file_channels: &dyn Fn(&str) -> Option<usize>) -> Option<usize> {
    if !track.output_stereo {
        return None;
    }
    file_channels(&track.file_path).filter(|&n| n >= 3)
}

/// 채널 게이트(0부터, 길이 `len`): 켠 모노 k → k-1, 켠 스테레오 k → k-1·k, 켠 멀티 k → k-1부터 그 자리를 쓰는 다채널
/// 트랙의 가장 긴 파일 채널 수만큼(트랙이 없으면 k-1 하나). 0번 키와 `len` 밖은 뺀다. 켠 것이 없으면 모두 닫힘(소리 없음,
/// 2026-11-21). `file_channels`는 음원 파일의 채널 수(읽지 못하면 None — 그 트랙은 재생되지 않으므로 뺀다).
pub fn open_channels_with(config: &AppConfig, len: usize, file_channels: &dyn Fn(&str) -> Option<usize>) -> Vec<bool> {
    let mut open = vec![false; len];
    let mut set_range = |start: usize, count: usize| {
        for slot in open.iter_mut().skip(start).take(count) {
            *slot = true;
        }
    };
    for (&key, setting) in &config.mono_configs {
        if setting.enabled && key >= 1 {
            set_range((key - 1) as usize, 1);
        }
    }
    for (&key, setting) in &config.stereo_configs {
        if setting.enabled && key >= 1 {
            set_range((key - 1) as usize, 2);
        }
    }
    for (&key, setting) in &config.multi_configs {
        if setting.enabled && key >= 1 {
            let start = (key - 1) as usize;
            let span = config
                .rooms
                .iter()
                .flat_map(|room| room.tracks.iter())
                .filter(|track| track.output_channel as usize == start)
                .filter_map(|track| multi_span(track, file_channels))
                .max()
                .unwrap_or(1);
            set_range(start, span);
        }
    }
    open
}

/// [`open_channels_with`]를 음원 파일 채널 수 기억([`cached_file_channels`])으로 부른다. 설정을 읽거나 바꿀 때, 엔진을
/// 시작할 때 부른다(오디오 스레드 밖).
pub fn open_channels(config: &AppConfig, len: usize) -> Vec<bool> {
    open_channels_with(config, len, &cached_file_channels)
}

/// 파일 채널 수. 메모리에 읽은 음원이면 그 값, 아니면 머리말을 읽고 경로·크기·수정 시각으로 기억한다(설정이 바뀔
/// 때마다 다시 읽지 않게). 오디오 스레드 밖에서만 부른다(파일 I/O·잠금). 파일을 열지 못하면 None.
pub fn cached_file_channels(path: &str) -> Option<usize> {
    use std::collections::HashMap;
    use std::sync::Mutex;
    type Key = (u64, Option<std::time::SystemTime>);
    type Cache = Option<HashMap<String, (Key, Option<usize>)>>;
    static CACHE: Mutex<Cache> = Mutex::new(None);

    let state = &crate::core::state::GLOBAL_STATE;
    for loaded in [&state.preloaded_sounds, &state.sound_cache] {
        let channels = loaded
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(path)
            .map(|data| data.channels as usize)
            .filter(|&n| n > 0);
        if channels.is_some() {
            return channels;
        }
    }
    let meta = std::fs::metadata(path).ok()?;
    let key: Key = (meta.len(), meta.modified().ok());
    let mut guard = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let cache = guard.get_or_insert_with(HashMap::new);
    if let Some((cached_key, channels)) = cache.get(path) {
        if *cached_key == key {
            return *channels;
        }
    }
    let channels = Some(crate::audio::player::SoundData::probe_channels(std::path::Path::new(path)) as usize)
        .filter(|&n| n > 0);
    cache.insert(path.to_string(), (key, channels));
    channels
}
