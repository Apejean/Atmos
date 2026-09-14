import 'package:flutter_test/flutter_test.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';

void main() {
  group('buildChannelPositionsPayload', () {
    test('returns null for channels without a speaker node', () {
      final payload = buildChannelPositionsPayload(const [], 4);
      expect(payload, [null, null, null, null]);
    });

    test('x/y는 이미 미터이므로 변환 없이 그대로 실린다', () {
      // 이 테스트는 예전에 "픽셀을 미터로 변환한다"를 검증했다. 그 전제가
      // 틀렸다. SpeakerNode.x/y는 생성 시점부터 미터다
      // (dynamic_3d_room.dart의 `x: roomWidth * 0.25`), 3D 룸 드래그도
      // 미터로 되돌려준다(studio_engine.html의 `x = newPos.x + width/2`).
      // 픽셀로 오해해 50으로 나누던 탓에 엔진에 가는 좌표가 50배 작아졌고,
      // 절대 거리가 필요한 계산(초기반사음 경로, 거리 감쇠, 시간 정렬)이
      // 전부 무의미해졌다.
      final node = SpeakerNode(id: 'spk_0', x: 10.0, y: 5.0, channel: 0, heightZ: 3.5);

      final entry = buildChannelPositionsPayload([node], 1)[0]!;

      expect(entry['x'], 10.0, reason: '미터 좌표를 다시 변환하면 안 된다');
      expect(entry['y'], 5.0, reason: '미터 좌표를 다시 변환하면 안 된다');
      expect(entry['z'], 3.5, reason: 'heightZ도 이미 미터다');
    });

    test('preserves full height/rotation/tilt/dispersion schema, not zeroed out', () {
      final node = SpeakerNode(
        id: 'spk_0',
        x: 10.0,
        y: 20.0,
        channel: 0,
        heightZ: 3.5,
        rotation: 45.0,
        pitchTilt: 12.5,
        dispersionAngle: 90.0,
      );

      final payload = buildChannelPositionsPayload([node], 1);

      expect(payload.length, 1);
      final entry = payload[0]!;
      expect(entry['x'], 10.0);
      expect(entry['y'], 20.0);
      expect(entry['z'], 3.5, reason: 'z must come from heightZ, not hardcoded 0.0');
      expect(entry['yaw_rotation'], 45.0, reason: 'rotation must not be silently reset to 0');
      expect(entry['pitch_tilt'], 12.5, reason: 'pitchTilt must not be silently reset to 0');
      expect(entry['dispersion_angle'], 90.0, reason: 'dispersionAngle must not be silently reset to 0');
    });
  });
}
