//! 믹서가 처리할 출력 채널 수(처리 폭).
//!
//! 장치는 지원하는 최대 채널로 연다(WASAPI·CoreAudio는 채널 수를 줄여 열면 실패할 수 있다). 대신
//! 믹서는 출력 설정에서 켠 가장 높은 채널까지만 처리한다. 채널마다 크로스오버·채널 DSP·리미터가
//! 샘플 단위로 돌아 처리 시간이 채널 수에 비례하기 때문이다. 2026-10-08 Windows에서 RME MADIface USB
//! (출력 94채널)를 멀티 그룹 규칙 때문에 전부 처리해, 48kHz·1024프레임 예산(21.3ms)의 77%를 믹서가
//! 쓰고 최악에는 넘겨 소리가 끊기고 느려졌다(출력 설정에서는 CH40까지만 켜 둔 상태).
use crate::common::config::AppConfig;

/// 처리할 채널 수(1-based 번호로 센 가장 높은 채널). 결과는 `hw_channels`를 넘지 않는다.
///
/// - 출력 설정에서 켠 그룹이 없으면 장치 전체(`compute_enabled_channels`의 "전체 개방"과 같다).
/// - Mono k → k, Stereo k → k+1(장치 끝을 넘지 않음), Multi k → k. 장치 채널 수를 넘는 번호
///   (예전 화면이 남긴 255 등)는 세지 않는다.
/// - 가장 높은 것이 Multi 시작 채널뿐이면(그 위로 Mono·Stereo가 없으면) 다채널 길이는 재생하는
///   파일의 채널 수로 정해져 알 수 없으므로 장치 끝까지 처리한다(지금 동작 유지).
/// - 범위 안에 켠 채널이 하나도 없으면 지금처럼 장치 전체.
pub fn processing_channel_count(config: &AppConfig, hw_channels: usize) -> usize {
    if hw_channels == 0 {
        return 0;
    }
    let in_range = |k: u32| k >= 1 && (k as usize) <= hw_channels;
    let mut any_enabled = false;
    let mut highest_fixed = 0usize;
    let mut highest_multi = 0usize;
    for (&k, setting) in &config.mono_configs {
        if setting.enabled {
            any_enabled = true;
            if in_range(k) {
                highest_fixed = highest_fixed.max(k as usize);
            }
        }
    }
    for (&k, setting) in &config.stereo_configs {
        if setting.enabled {
            any_enabled = true;
            if in_range(k) {
                highest_fixed = highest_fixed.max((k as usize + 1).min(hw_channels));
            }
        }
    }
    for (&k, setting) in &config.multi_configs {
        if setting.enabled {
            any_enabled = true;
            if in_range(k) {
                highest_multi = highest_multi.max(k as usize);
            }
        }
    }
    if !any_enabled || (highest_fixed == 0 && highest_multi == 0) || highest_multi > highest_fixed {
        return hw_channels;
    }
    highest_fixed
}

/// 믹서 폭. 2채널 장치도 믹서는 16채널 확장 버스로 돌아 왔으므로(엔진의 기존 동작) 최소 16이다.
pub fn mixer_width(processing_channels: usize) -> usize {
    16.max(processing_channels)
}

/// 출력 설정에서 켠 그룹(모노·스테레오·멀티)이 하나도 없는가 — "전체 개방"(`compute_enabled_channels`와 같은 기준).
fn no_group_enabled(config: &AppConfig) -> bool {
    !config.mono_configs.values().any(|s| s.enabled)
        && !config.stereo_configs.values().any(|s| s.enabled)
        && !config.multi_configs.values().any(|s| s.enabled)
}

/// 트랙과 서브우퍼가 쓰는 가장 높은 채널(1부터 센 번호, 장치 채널 수로 자르고 최소 2). 출력 설정이 비었을 때 처리 폭
/// (설계 docs/superpowers/specs/2026-11-20-processing-width-fallback-design.md). 믹서의 라우팅 규칙(audio/mixer.rs)과
/// 같다: 스테레오 출력을 끄면 다채널도 모노로 합쳐 `output_channel` 하나, 켜면 파일 채널 수만큼(모노 파일은 2 — 모노→스테레오
/// 올림). 트랙의 `output_channel`은 0부터 센 장치 채널(CH1 = 0, 믹서 `hw_ch = output_channel + ch`)이라 가장 높은 채널은
/// `output_channel + 폭`이다(1.3.14는 1부터 센다고 보고 맨 위 채널 하나를 빠뜨렸다, 2026-11-21). `file_channels`가
/// None이면(읽지 못하는 파일) 재생되지 않으므로 뺀다. `highest_sub`은 서브우퍼로 지정한 가장 높은 채널(1부터, 0이면 없음)
/// — 베이스 매니지먼트가 트랙이 없는 서브 채널로도 저역을 보낸다.
pub fn used_channel_ceiling(
    config: &AppConfig,
    hw_channels: usize,
    file_channels: &dyn Fn(&str) -> Option<usize>,
    highest_sub: usize,
) -> usize {
    let mut top = highest_sub;
    for track in config.rooms.iter().flat_map(|r| r.tracks.iter()) {
        let start = track.output_channel as usize;
        let span = if track.output_stereo {
            match file_channels(&track.file_path) {
                Some(1) => 2,
                Some(n) => n,
                None => continue,
            }
        } else {
            1
        };
        top = top.max(start + span);
    }
    top.max(2).min(hw_channels)
}

/// 실제로 쓸 처리 폭. 켠 그룹이 없으면 `used_ceiling`(트랙·서브 기준, [`used_channel_ceiling`]), 있으면
/// [`processing_channel_count`] 그대로. 예전에는 켠 그룹이 없으면 장치 전체(94채널)를 처리해 예산을 넘었다(2026-11-20).
pub fn effective_processing_channels(config: &AppConfig, hw_channels: usize, used_ceiling: usize) -> usize {
    if hw_channels == 0 {
        return 0;
    }
    if no_group_enabled(config) {
        return used_ceiling.clamp(1, hw_channels);
    }
    processing_channel_count(config, hw_channels)
}

/// 지금 설정·음원·스피커 배치로 정한 처리 폭(엔진 시작, 설정 변경, 스피커 배치 변경에서 부른다 — 오디오 스레드 밖).
/// 켠 그룹이 있으면 파일을 읽지 않고 [`processing_channel_count`] 그대로다.
pub fn current_processing_channels(config: &AppConfig, hw_channels: usize) -> usize {
    if !no_group_enabled(config) {
        return processing_channel_count(config, hw_channels);
    }
    let sub = crate::core::state::GLOBAL_STATE
        .highest_sub_channel
        .load(std::sync::atomic::Ordering::Relaxed) as usize;
    let ceiling = used_channel_ceiling(config, hw_channels, &cached_file_channels, sub);
    effective_processing_channels(config, hw_channels, ceiling)
}

/// 지금 설정이 정하는 믹서 폭이 엔진이 연 믹서 폭과 다르면 `Some((연 폭, 원하는 폭))`. 엔진이 없거나 장치 채널 수를
/// 모르면 None(오디오 스레드 밖에서 부른다 — 파일 머리말을 읽을 수 있다).
pub fn width_change(config: &AppConfig) -> Option<(usize, usize)> {
    use std::sync::atomic::Ordering;
    let state = &crate::core::state::GLOBAL_STATE;
    let hw_channels = state.active_device_channels.load(Ordering::SeqCst) as usize;
    let current = state.processing_channels.load(Ordering::SeqCst) as usize;
    if hw_channels == 0 || current == 0 {
        return None;
    }
    let wanted = mixer_width(current_processing_channels(config, hw_channels));
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

/// 파일 채널 수. 미리 읽은 음원이면 그 값, 아니면 머리말을 읽고 경로·크기·수정 시각으로 기억한다(설정이 바뀔 때마다
/// 다시 읽지 않게). 읽지 못하면 None — 그 트랙은 재생되지 않으므로 처리 폭 계산에서 빠진다(앱 로그에 경로마다 한 번).
fn cached_file_channels(path: &str) -> Option<usize> {
    use std::collections::HashMap;
    use std::sync::Mutex;
    type Key = (u64, Option<std::time::SystemTime>);
    type Cache = Option<HashMap<String, (Key, Option<usize>)>>;
    static CACHE: Mutex<Cache> = Mutex::new(None);

    let preloaded = crate::core::state::GLOBAL_STATE
        .preloaded_sounds
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .get(path)
        .map(|d| d.channels as usize)
        .filter(|&n| n > 0);
    if preloaded.is_some() {
        return preloaded;
    }
    let key: Key = match std::fs::metadata(path) {
        Ok(m) => (m.len(), m.modified().ok()),
        Err(_) => (0, None),
    };
    let mut guard = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let cache = guard.get_or_insert_with(HashMap::new);
    if let Some((k, n)) = cache.get(path) {
        if *k == key {
            return *n;
        }
    }
    let n = crate::audio::streaming::file_channels(path);
    if n.is_none() {
        crate::core::state::GLOBAL_STATE
            .log(format!("처리 폭 계산: 파일 채널 수를 읽지 못해 이 트랙은 뺀다 — {path}"));
    }
    cache.insert(path.to_string(), (key, n));
    n
}
