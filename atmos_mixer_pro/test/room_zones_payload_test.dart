import 'package:flutter_test/flutter_test.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';

void main() {
  group('buildRoomZonesPayload', () {
    test('uses ceilingHeight for boundary_max.z and includes ear_level/absorption_coeff', () {
      final room = RoomZone(
        id: 'room_1',
        // 사각형 필드는 옛 2D 캔버스 잔재이며 페이로드에 쓰이지 않는다.
        // 일부러 물리 치수와 다른 값을 넣어 무시되는지 확인한다.
        x: 200.0,
        y: 200.0,
        width: 100.0,
        height: 100.0,
        color: 0xFFFFFFFF,
        physicalWidth: 8.0,
        physicalHeight: 6.0,
        ceilingHeight: 4.0,
        earLevel: 1.5,
        absorptionCoeff: 0.2,
      );

      final entry = buildRoomZonesPayload([room])[0];

      // 3D 룸은 방을 physicalWidth x physicalHeight 미터로 원점에 짓고,
      // 스피커는 그 안의 0..W 좌표를 갖는다. 경계도 같은 공간이어야 한다.
      expect(entry['boundary_min']['x'], 0.0);
      expect(entry['boundary_min']['y'], 0.0);
      expect(entry['boundary_max']['x'], 8.0,
          reason: '경계는 physicalWidth를 따라야 한다(사각형 width가 아니라)');
      expect(entry['boundary_max']['y'], 6.0,
          reason: '경계는 physicalHeight를 따라야 한다');
      expect(entry['boundary_max']['z'], 4.0, reason: 'boundary_max.z must come from ceilingHeight, not hardcoded 2.0');
      expect(entry['ear_level'], 1.5, reason: 'ear_level must be sent so early reflection listener height is correct');
      expect(entry['absorption_coeff'], 0.2);
    });
  });
}
