import 'package:atmos_mixer_pro/src/rust/common/config.dart';

/// Output Config 창의 세 맵(모노·스테레오·멀티, 키는 1부터 — `config.json`과 같다).
///
/// 세 줄은 열린 채널 하나를 보여 준다(사용자 결정 2026-11-21 "채널 하나로 묶기", 설계
/// `docs/superpowers/specs/2026-11-21-output-config-open-channels-design.md`): 모노 k = CHk 열림, 스테레오 k/k+1 =
/// 두 채널 모두 열림, 멀티 k 시작 = CHk 열림. 열린 채널은 `channel_routing.dart` `openOutputChannels`·엔진
/// `core::output_groups::open_channels`와 같은 규칙이다.
class OutputGroupMaps {
  final Map<int, ChannelSetting> mono;
  final Map<int, ChannelSetting> stereo;
  final Map<int, ChannelSetting> multi;

  const OutputGroupMaps({required this.mono, required this.stereo, required this.multi});

  /// 열린 채널(0부터).
  Set<int> openChannels() {
    final open = <int>{};
    mono.forEach((k, s) {
      if (s.enabled && k >= 1) open.add(k - 1);
    });
    stereo.forEach((k, s) {
      if (s.enabled && k >= 1) open.addAll([k - 1, k]);
    });
    multi.forEach((k, s) {
      if (s.enabled && k >= 1) open.add(k - 1);
    });
    return open;
  }

  /// 모노 줄 CH[key]가 켜져 보이는가.
  bool isMonoOn(int key) => openChannels().contains(key - 1);

  /// 스테레오 줄 [key]/[key]+1이 켜져 보이는가.
  bool isStereoOn(int key) {
    final open = openChannels();
    return open.contains(key - 1) && open.contains(key);
  }

  /// 멀티 줄 CH[key] 시작이 켜져 보이는가.
  bool isMultiOn(int key) => openChannels().contains(key - 1);
}

ChannelSetting _withEnabled(ChannelSetting s, bool enabled) => ChannelSetting(
  enabled: enabled,
  customName: s.customName,
  delayMs: s.delayMs,
  eqBands: s.eqBands,
  position: s.position,
  phaseInvert: s.phaseInvert,
  gainDb: s.gainDb,
);

const _freshMono = ChannelSetting(
  enabled: true,
  customName: '',
  delayMs: 0.0,
  eqBands: [],
  phaseInvert: false,
  gainDb: 0.0,
);

/// [channels](0부터)를 열거나 닫은 새 맵을 돌려준다(원래 맵은 그대로).
///
/// - 열기: 닫혀 있던 채널마다 모노 항목을 켠다(없으면 새로 만든다).
/// - 닫기: 그 채널을 덮는 모든 항목(모노·스테레오·멀티)을 끄고, 꺼진 항목이 덮던 다른 채널은 모노 항목을 켜서 열린
///   채로 둔다.
/// - 새 항목은 모노로만 만든다: 엔진은 항목의 채널 보정값(지연·EQ·위상·게인·위치)을 켜짐과 상관없이 모노 → 스테레오 →
///   멀티(6채널) 순서로 덮어 적용한다(`audio/mixer.rs`). 스테레오·멀티 항목을 새로 만들면 기존 보정값을 기본값으로
///   덮지만, 모노는 가장 먼저 적용되어 영향이 없다.
/// - 켜고 끌 때 항목의 다른 값은 그대로 둔다(예전 창은 누를 때마다 위상 반전·게인을 기본값으로 되돌리고 위치를 지웠다).
OutputGroupMaps setOutputChannelsOpen(OutputGroupMaps maps, Set<int> channels, bool open) {
  final mono = Map<int, ChannelSetting>.of(maps.mono);
  final stereo = Map<int, ChannelSetting>.of(maps.stereo);
  final multi = Map<int, ChannelSetting>.of(maps.multi);
  final before = maps.openChannels();

  void openMono(int ch0) {
    final key = ch0 + 1;
    final existing = mono[key];
    mono[key] = existing == null ? _freshMono : _withEnabled(existing, true);
  }

  if (open) {
    for (final ch0 in channels) {
      if (!before.contains(ch0)) openMono(ch0);
    }
  } else {
    void closeWhere(Map<int, ChannelSetting> map, bool Function(int key) covers) {
      for (final key in map.keys.toList()) {
        final s = map[key]!;
        if (s.enabled && covers(key)) map[key] = _withEnabled(s, false);
      }
    }

    closeWhere(mono, (k) => channels.contains(k - 1));
    closeWhere(stereo, (k) => channels.contains(k - 1) || channels.contains(k));
    closeWhere(multi, (k) => channels.contains(k - 1));
    final after = OutputGroupMaps(mono: mono, stereo: stereo, multi: multi).openChannels();
    for (final ch0 in before.difference(channels).difference(after)) {
      openMono(ch0);
    }
  }
  return OutputGroupMaps(mono: mono, stereo: stereo, multi: multi);
}
