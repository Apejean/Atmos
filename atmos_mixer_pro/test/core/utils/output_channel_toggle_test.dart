import 'package:flutter_test/flutter_test.dart';
import 'package:atmos_mixer_pro/core/utils/output_channel_toggle.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';

/// Output Config 창 켜고 끄기(사용자 결정 2026-11-22 "이전처럼 에이블톤처럼 따로 하자", 설계
/// docs/superpowers/specs/2026-11-22-output-config-separate-rows-design.md).
/// 줄 하나를 누르면 그 줄만 바뀐다. 1.3.15·1.3.16은 세 줄을 열린 채널 하나로 묶어, 모노 CH2를 껐다 켜면 모노 CH2만 다시
/// 켜지고 스테레오 1/2·멀티 CH2 시작은 파일에 꺼진 채 남았다(창에는 켜져 보였다, 2026-11-22 1.3.16 설치본 확인).
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

  test('줄 하나를 끄고 다시 켜면 그 줄만 바뀌고 처음과 같아진다', () {
    final start = {1: setting(), 2: setting()};
    final off = setOutputRow(start, 2, false);
    expect(off[1]!.enabled, isTrue);
    expect(off[2]!.enabled, isFalse);
    final on = setOutputRow(off, 2, true);
    expect(on[2]!.enabled, isTrue);
    expect(on.keys.toSet(), {1, 2});
  });

  test('없던 줄을 켜면 새 항목을 만들고, 없던 줄을 꺼도 항목을 만들지 않는다', () {
    final on = setOutputRow({}, 3, true);
    expect(on[3]!.enabled, isTrue);
    expect(on[3]!.gainDb, 0.0);
    expect(setOutputRow({}, 3, false), isEmpty);
  });

  test('켜고 끌 때 이름·위상 반전·게인은 그대로 둔다', () {
    final start = {5: setting(gainDb: -3.0, phaseInvert: true, name: 'Sub')};
    final off = setOutputRow(start, 5, false);
    expect(off[5]!.gainDb, -3.0);
    expect(off[5]!.phaseInvert, isTrue);
    expect(off[5]!.customName, 'Sub');
    final on = setOutputRow(off, 5, true);
    expect(on[5]!.gainDb, -3.0);
    expect(on[5]!.phaseInvert, isTrue);
  });

  test('원래 맵은 바꾸지 않는다', () {
    final start = {1: setting()};
    setOutputRow(start, 1, false);
    expect(start[1]!.enabled, isTrue);
  });
}
