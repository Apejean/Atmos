// tests/test_enabled_channels_multi_config.rs
// Atmos Mixer Pro - enabled_channels(믹서 출력 게이트) 계산 회귀 테스트
//
// 배경: `simple.rs`의 enabled_channels 계산 블록이 mono_configs/stereo_configs만 순회하고
// multi_configs를 빠뜨려, Multi 그룹으로만 열어둔 채널이 튜닝은 적용되지만 출력 게이트에서
// 막혀 무음이 되는 결함이 있었다. `compute_enabled_channels()`로 로직을 순수 함수로 추출해
// 이 결함을 기계적으로 검증한다 (OUTPUT_CHANNEL_MAPPING_UNIFICATION_SPEC.md 3.1절).

use rust_lib_atmos_mixer_pro::api::simple::compute_enabled_channels;
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, ChannelSetting};

/// 테스트용 ChannelSetting을 만든다 (delay/eq/phase/gain은 이 테스트와 무관하므로 기본값).
fn setting(enabled: bool) -> ChannelSetting {
    ChannelSetting {
        enabled,
        custom_name: String::new(),
        delay_ms: 0.0,
        eq_bands: Vec::new(),
        position: None,
        phase_invert: false,
        gain_db: 0.0,
    }
}

/// 세 맵이 모두 비어있으면 어느 채널도 열지 않는다(사용자 결정 2026-11-21 "아무것도 안 켜면 소리 없음",
/// 설계 docs/superpowers/specs/2026-11-21-empty-output-config-silent-design.md). 2026-09-13부터 이 경우를
/// "전체 개방"으로 봤다.
#[test]
fn all_configs_empty_opens_no_channel() {
    let config = AppConfig::default();
    let enabled = compute_enabled_channels(&config, 8);
    assert_eq!(enabled, vec![false; 8]);
}

/// mono_configs만 설정된 경우: key(1-based)마다 real_ch, real_ch+1 (0-based) 두 채널이 열린다.
#[test]
fn mono_only_opens_single_channel() {
    let mut config = AppConfig::default();
    config.mono_configs.insert(1, setting(true)); // 1-based key=1 -> 0-based ch 0
    let enabled = compute_enabled_channels(&config, 8);
    // Mono 그룹 하나 = 채널 하나. 예전에는 ch0과 ch1을 함께 열어서, 사용자가
    // Mono 1만 열어도 채널 2가 열렸다. Dart의 라우팅 항목 생성기도 같은 가정
    // 때문에 연속한 키에서 같은 채널을 중복 노출했다. 양쪽을 1:1로 통일했다.
    let expected = vec![true, false, false, false, false, false, false, false];
    assert_eq!(enabled, expected);
}

/// 연속한 Mono 키를 열면 그 채널들만 정확히 열린다(페어로 번지지 않는다).
#[test]
fn consecutive_mono_keys_open_exactly_those_channels() {
    let mut config = AppConfig::default();
    config.mono_configs.insert(1, setting(true)); // -> ch0
    config.mono_configs.insert(2, setting(true)); // -> ch1
    let enabled = compute_enabled_channels(&config, 8);
    let expected = vec![true, true, false, false, false, false, false, false];
    assert_eq!(enabled, expected);
}

/// stereo_configs만 설정된 경우: key(1-based)마다 real_ch, real_ch+1 (0-based) 두 채널이 열린다.
#[test]
fn stereo_only_opens_pair() {
    let mut config = AppConfig::default();
    config.stereo_configs.insert(3, setting(true)); // 1-based key=3 -> 0-based ch 2,3
    let enabled = compute_enabled_channels(&config, 8);
    let expected = vec![false, false, true, true, false, false, false, false];
    assert_eq!(enabled, expected);
}

/// multi_configs만 설정된 경우 (회귀 대상): 시작 채널(0-based)부터 하드웨어 끝까지 전부 열려야 한다.
/// 수정 전 코드는 이 케이스에서 mono/stereo가 비어있으므로 "전 채널 개방" 분기로 우연히 통과했었다.
/// 이 테스트는 그 우연이 아니라 multi_configs가 실제로 인식되는지를 검증하기 위해,
/// 아래 mono+multi 혼합 케이스와 짝을 이룬다.
#[test]
fn multi_only_opens_from_start_to_hw_end() {
    let mut config = AppConfig::default();
    config.multi_configs.insert(5, setting(true)); // 1-based key=5 -> 0-based ch 4부터 끝까지
    let enabled = compute_enabled_channels(&config, 8);
    let expected = vec![false, false, false, false, true, true, true, true];
    assert_eq!(enabled, expected);
}

/// mono + multi 혼합 (기존 버그가 드러나는 케이스):
/// 수정 전 코드는 mono_configs가 비어있지 않으므로 else 분기로 들어가지만 multi_configs를
/// 순회하지 않아 채널 4~7이 계속 false로 남았다 (무음 버그). 수정 후에는 mono가 여는 0과
/// multi가 여는 4..8이 모두 true여야 한다.
#[test]
fn mono_and_multi_mixed_opens_both_ranges() {
    let mut config = AppConfig::default();
    config.mono_configs.insert(1, setting(true)); // 0-based ch 0 (Mono = 채널 하나)
    config.multi_configs.insert(5, setting(true)); // 0-based ch 4..8
    let enabled = compute_enabled_channels(&config, 8);
    let expected = vec![true, false, false, false, true, true, true, true];
    assert_eq!(
        enabled, expected,
        "multi_configs가 반영되지 않으면 채널 4~7이 열리지 않는다 (실제 버그 재현)"
    );
}

/// enabled=false인 그룹은 열리지 않는다 (mono/stereo와 동일하게 multi도 enabled 플래그를 존중해야 함).
#[test]
fn all_groups_disabled_opens_no_channel() {
    let mut config = AppConfig::default();
    config.multi_configs.insert(1, setting(false));
    let enabled = compute_enabled_channels(&config, 8);
    // 항목은 있지만 전부 enabled=false다 — 켠 그룹이 없으니 비어 있는 것과 같다. 화면
    // (buildChannelRoutingItems)도 같은 기준으로 고를 채널을 내놓지 않는다(2026-09-13의
    // "화면은 채널을 내놓는데 엔진은 막는" 어긋남은 생기지 않는다).
    assert_eq!(enabled, vec![false; 8]);
}

/// 활성 그룹이 하나라도 있으면 그 그룹만 열린다.
#[test]
fn one_enabled_group_closes_the_rest() {
    let mut config = AppConfig::default();
    config.mono_configs.insert(1, setting(true)); // ch0만 개방
    config.mono_configs.insert(5, setting(false)); // 비활성이므로 무시
    config.multi_configs.insert(7, setting(false)); // 비활성이므로 무시
    let enabled = compute_enabled_channels(&config, 8);
    let expected = vec![true, false, false, false, false, false, false, false];
    assert_eq!(enabled, expected);
}

/// 하드웨어 채널 수를 넘는 키가 들어와도 패닉하지 않고 안전하게 무시해야 한다 (경계 안전).
#[test]
fn out_of_range_keys_are_safely_ignored() {
    let mut config = AppConfig::default();
    config.mono_configs.insert(100, setting(true)); // hw_len=8을 훨씬 초과
    config.multi_configs.insert(50, setting(true));
    let enabled = compute_enabled_channels(&config, 8);
    assert_eq!(enabled, vec![false; 8]);
    assert_eq!(enabled.len(), 8);
}

/// hw_len=0 (장치 미연결 등 극단 케이스)에서도 패닉하지 않아야 한다.
#[test]
fn zero_hw_len_does_not_panic() {
    let mut config = AppConfig::default();
    config.multi_configs.insert(1, setting(true));
    let enabled = compute_enabled_channels(&config, 0);
    assert_eq!(enabled, Vec::<bool>::new());
}
