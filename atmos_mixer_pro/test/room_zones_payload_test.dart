import 'package:flutter_test/flutter_test.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';

void main() {
  group('buildRoomZonesPayload', () {
    test('uses ceilingHeight for boundary_max.z and includes ear_level/absorption_coeff', () {
      final room = RoomZone(
        id: 'room_1',
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
        color: 0xFFFFFFFF,
        ceilingHeight: 4.0,
        earLevel: 1.5,
        absorptionCoeff: 0.2,
      );

      final entry = buildRoomZonesPayload([room], 50.0)[0];

      expect(entry['boundary_max']['z'], 4.0, reason: 'boundary_max.z must come from ceilingHeight, not hardcoded 2.0');
      expect(entry['ear_level'], 1.5, reason: 'ear_level must be sent so early reflection listener height is correct');
      expect(entry['absorption_coeff'], 0.2);
    });
  });
}
