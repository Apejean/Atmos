import 'package:flutter_test/flutter_test.dart';

import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';

/// 스피커 저역 한계(자동 EQ 로우컷의 기준)는 스피커마다 설정할 수 있어야 한다.
/// 예전에는 모든 스피커가 80Hz로 고정이었고 화면에서 바꿀 수 없었다.
void main() {
  test('새 스피커의 저역 한계 기본값은 45Hz다', () {
    // 일반 설치형 스피커가 40~60Hz까지 내는 수준이다. 80Hz면 서브 없이 쓸 때 저역이 빈다.
    final node = SpeakerNode(id: 'spk', x: 1.0, y: 1.0, channel: 0);
    expect(node.lowCutHz, 45.0);
  });

  test('사용자가 바꾼 저역 한계는 저장 후에도 남는다', () {
    final node = SpeakerNode(id: 'spk', x: 1.0, y: 1.0, channel: 0).copyWith(lowCutHz: 60.0);
    final restored = SpeakerNode.fromJson(node.toJson());
    expect(restored.lowCutHz, 60.0);
  });

  test('예전 저장본의 80Hz는 사용자가 정한 값이 아니므로 새 기본값으로 읽는다', () {
    // 예전 low_cut_hz는 화면에서 바꿀 수 없던 기본값(80Hz)이라, 그대로 이어받으면
    // 기존 스피커만 계속 저역이 잘린다.
    final legacy = {
      'id': 'spk_old',
      'x': 1.0,
      'y': 1.0,
      'channel': 0,
      'low_cut_hz': 80.0,
    };
    expect(SpeakerNode.fromJson(legacy).lowCutHz, 45.0);
  });
}
