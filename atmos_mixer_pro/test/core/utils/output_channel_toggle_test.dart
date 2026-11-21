import 'package:flutter_test/flutter_test.dart';
import 'package:atmos_mixer_pro/core/utils/output_channel_toggle.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';

/// Output Config 창 켜고 끄기(사용자 결정 2026-11-21 "채널 하나로 묶기", 설계
/// docs/superpowers/specs/2026-11-21-output-config-open-channels-design.md).
/// 세 줄은 열린 채널 하나를 보여 주고, 어느 줄에서 켜고 끄든 그 채널이 열리고 닫힌다. 새 항목은 모노로만 만든다 —
/// 엔진이 항목의 채널 보정값을 모노 → 스테레오 → 멀티 순서로 덮어 적용하므로 스테레오·멀티 항목을 새로 만들면 기존
/// 보정값을 기본값으로 덮는다.
void main() {
  ChannelSetting setting({bool enabled = true, double gainDb = 0.0, bool phaseInvert = false, String name = ''}) =>
      ChannelSetting(
        enabled: enabled,
        customName: name,
        delayMs: 0.0,
        eqBands: const [],
        phaseInvert: phaseInvert,
        gainDb: gainDb,
      );

  OutputGroupMaps maps({
    Map<int, ChannelSetting> mono = const {},
    Map<int, ChannelSetting> stereo = const {},
    Map<int, ChannelSetting> multi = const {},
  }) => OutputGroupMaps(mono: Map.of(mono), stereo: Map.of(stereo), multi: Map.of(multi));

  test('스테레오 1/2를 켜면 모노 CH1·CH2와 멀티 CH1·CH2 시작도 켜져 보인다', () {
    final next = setOutputChannelsOpen(maps(), {0, 1}, true);
    expect(next.openChannels(), {0, 1});
    expect(next.isMonoOn(1), isTrue);
    expect(next.isMonoOn(2), isTrue);
    expect(next.isStereoOn(1), isTrue);
    expect(next.isMultiOn(1), isTrue);
    expect(next.isMultiOn(2), isTrue);
    // 새 항목은 모노로만 만든다.
    expect(next.stereo, isEmpty);
    expect(next.multi, isEmpty);
    expect(next.mono.keys.toSet(), {1, 2});
  });

  test('모노 CH1 하나만 켜면 스테레오 1/2는 꺼진 채다(CH2가 닫힘)', () {
    final next = setOutputChannelsOpen(maps(), {0}, true);
    expect(next.isMonoOn(1), isTrue);
    expect(next.isMultiOn(1), isTrue);
    expect(next.isStereoOn(1), isFalse);
  });

  test('모노 CH2를 끄면 스테레오 1/2와 멀티 CH2 시작도 꺼지고, CH1은 열린 채로 남는다', () {
    // 스테레오 1/2 항목 하나로 CH1·CH2가 열려 있다.
    final start = maps(stereo: {1: setting(gainDb: -3.0)});
    final next = setOutputChannelsOpen(start, {1}, false);
    expect(next.openChannels(), {0});
    expect(next.isStereoOn(1), isFalse);
    expect(next.isMultiOn(2), isFalse);
    expect(next.isMonoOn(1), isTrue, reason: 'CH1은 닫지 않았다');
    // 스테레오 항목은 지우지 않고 끄기만 한다(보정값 보존). CH1은 모노 항목으로 열린다.
    expect(next.stereo[1]!.enabled, isFalse);
    expect(next.stereo[1]!.gainDb, -3.0);
    expect(next.mono[1]!.enabled, isTrue);
  });

  test('켜고 끌 때 위상 반전·게인 등 다른 보정값은 그대로다', () {
    final start = maps(mono: {3: setting(enabled: false, gainDb: -6.0, phaseInvert: true, name: 'Rear')});
    final opened = setOutputChannelsOpen(start, {2}, true);
    expect(opened.mono[3]!.enabled, isTrue);
    expect(opened.mono[3]!.gainDb, -6.0);
    expect(opened.mono[3]!.phaseInvert, isTrue);
    expect(opened.mono[3]!.customName, 'Rear');
    final closed = setOutputChannelsOpen(opened, {2}, false);
    expect(closed.mono[3]!.enabled, isFalse);
    expect(closed.mono[3]!.gainDb, -6.0);
    expect(closed.mono[3]!.phaseInvert, isTrue);
  });

  test('멀티 CH1 시작을 켜면 CH1 하나만 열린다', () {
    final next = setOutputChannelsOpen(maps(), {0}, true);
    expect(next.openChannels(), {0});
  });

  test('이미 열린 채널을 켜도 항목을 새로 만들지 않는다', () {
    final start = maps(multi: {1: setting()});
    final next = setOutputChannelsOpen(start, {0}, true);
    expect(next.mono, isEmpty);
    expect(next.openChannels(), {0});
  });

  test('원래 맵은 바꾸지 않는다', () {
    final start = maps(mono: {1: setting()});
    setOutputChannelsOpen(start, {0}, false);
    expect(start.mono[1]!.enabled, isTrue);
  });
}
