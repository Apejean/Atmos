import 'package:atmos_mixer_pro/src/rust/common/config.dart';

ChannelSetting _withEnabled(ChannelSetting s, bool enabled) => ChannelSetting(
  enabled: enabled,
  customName: s.customName,
  delayMs: s.delayMs,
  eqBands: s.eqBands,
  position: s.position,
  phaseInvert: s.phaseInvert,
  gainDb: s.gainDb,
);

const _freshRow = ChannelSetting(
  enabled: true,
  customName: '',
  delayMs: 0.0,
  eqBands: [],
  phaseInvert: false,
  gainDb: 0.0,
);

/// Output Config 창의 줄 하나(모노 CHk·스테레오 k/k+1·멀티 CHk 시작, [key]는 1부터 — `config.json`과 같다)를 켜거나 끈
/// 새 맵을 돌려준다(원래 맵은 그대로).
///
/// 줄은 종류마다 따로 켜고 끈다(Ableton처럼, 사용자 결정 2026-11-22 "이전처럼 에이블톤처럼 따로 하자", 설계
/// `docs/superpowers/specs/2026-11-22-output-config-separate-rows-design.md`). 창은 각 줄의 저장된 `enabled`를 그대로
/// 보여 준다. 1.3.15·1.3.16은 세 줄을 열린 채널 하나로 묶어 보여 줘, 창에는 켜져 보이는 줄이 파일에는 꺼져 있을 수 있었다.
///
/// - 없던 줄을 켜면 새 항목을 만든다. 엔진은 항목의 채널 보정값(지연·EQ·위상·게인·위치)을 켜짐과 상관없이 모노 → 스테레오 →
///   멀티(6채널) 순서로 덮어 적용하므로, 새 스테레오·멀티 항목은 그 채널의 모노 보정값을 기본값으로 덮을 수 있다(예전부터
///   있던 문제, 남은 일).
/// - 켜고 끌 때 항목의 다른 값은 그대로 둔다(2026-11-21 고친 것 — 예전 창은 누를 때마다 위상 반전·게인을 기본값으로
///   되돌리고 위치를 지웠다).
Map<int, ChannelSetting> setOutputRow(Map<int, ChannelSetting> map, int key, bool on) {
  final next = Map<int, ChannelSetting>.of(map);
  final existing = next[key];
  if (existing != null) {
    next[key] = _withEnabled(existing, on);
  } else if (on) {
    next[key] = _freshRow;
  }
  return next;
}
