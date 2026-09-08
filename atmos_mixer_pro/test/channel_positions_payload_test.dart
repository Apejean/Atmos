import 'package:flutter_test/flutter_test.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';

void main() {
  group('buildChannelPositionsPayload', () {
    test('returns null for channels without a speaker node', () {
      final payload = buildChannelPositionsPayload(const [], 4, 50.0);
      expect(payload, [null, null, null, null]);
    });

    test('converts pixel x/y to meters while leaving heightZ in meters', () {
      final node = SpeakerNode(id: 'spk_0', x: 500.0, y: 250.0, channel: 0, heightZ: 3.5);

      final entry = buildChannelPositionsPayload([node], 1, 50.0)[0]!;

      // 50 px/m 이므로 500px -> 10m, 250px -> 5m.
      expect(entry['x'], 10.0, reason: 'x must be converted to meters for the DSP distance math');
      expect(entry['y'], 5.0, reason: 'y must be converted to meters for the DSP distance math');
      expect(entry['z'], 3.5, reason: 'heightZ is already in meters and must not be scaled');
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

      final payload = buildChannelPositionsPayload([node], 1, 1.0);

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
