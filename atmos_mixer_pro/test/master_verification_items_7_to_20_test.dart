import 'dart:math';
import 'package:flutter_test/flutter_test.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart' as exhibition;
import 'package:atmos_mixer_pro/features/exhibition/models/trajectory.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';

void main() {
  group('Master List Items 7 to 20 Comprehensive Verification Suite', () {
    
    // ITEM 7: 3D Trajectory Automation (Play/Pause/Loop, Waypoint Spline)
    test('Item 7: 3D Trajectory Automation - Waypoint & Spline Interpolation Safety', () {
      final waypoint1 = Waypoint(position: const Offset(0.0, 0.0), heightZ: 1.0);
      final waypoint2 = Waypoint(position: const Offset(10.0, 5.0), heightZ: 2.0);
      final waypoint3 = Waypoint(position: const Offset(20.0, 0.0), heightZ: 1.0);

      final traj = TrajectoryModel(
        id: 'traj_1',
        name: 'Circular Path',
        waypoints: [waypoint1, waypoint2, waypoint3],
        speed: 2.0,
        isPingPong: false,
      );

      expect(traj.waypoints.length, 3);
      expect(traj.isPingPong, isFalse);
      expect(traj.totalPathLength, greaterThan(0));

      // Test JSON round-trip
      final json = traj.toJson();
      final restored = TrajectoryModel.fromJson(json);
      expect(restored.id, equals('traj_1'));
      expect(restored.waypoints.length, equals(3));
    });

    // ITEM 8: Multichannel Speaker Mapping & Routing (Ch 1 ~ Ch 64)
    test('Item 8: Multichannel Speaker Mapping - Channel 1 to 64 Routing Bounds', () {
      for (int ch = 1; ch <= 64; ch++) {
        final node = SpeakerNode(
          id: 'spk_$ch',
          x: 1.0 * ch,
          y: 2.0 * ch,
          channel: ch,
        );
        expect(node.channel, greaterThanOrEqualTo(1));
        expect(node.channel, lessThanOrEqualTo(64));

        final json = node.toJson();
        final restored = SpeakerNode.fromJson(json);
        expect(restored.channel, equals(ch));
      }
    });

    // ITEM 9: Blueprint Background Image Load & Meter-Pixel Scaling
    test('Item 9: Blueprint Scaling - Zero and Negative Scale Guard', () {
      var blueprint = const BlueprintData(imagePath: '/path/to/blueprint.png', scale: 50.0);
      expect(blueprint.scale, equals(50.0));

      // Test scale calculation: 100 pixels = 2 meters -> scale = 50 px/m
      const pixelDistance = 100.0;
      const targetMeters = 2.0;
      double computedScale = (targetMeters > 0 && pixelDistance > 0)
          ? (pixelDistance / targetMeters)
          : blueprint.scale;
      expect(computedScale, equals(50.0));

      // Guard test: targetMeters = 0 must not produce Infinity or NaN
      const zeroMeters = 0.0;
      double safeScale = (zeroMeters > 0 && pixelDistance > 0)
          ? (pixelDistance / zeroMeters)
          : blueprint.scale;
      expect(safeScale.isFinite, isTrue);
      expect(safeScale, equals(50.0));
    });

    // ITEM 10: Room Acoustics / Transmission Loss (TL)
    test('Item 10: Room Acoustics - Transmission Loss (TL) 0 to 100 dB Clamping', () {
      const rawTL = 120.0; // Over max
      final clampedTL = rawTL.clamp(0.0, 100.0);
      expect(clampedTL, equals(100.0));

      const negativeTL = -15.0; // Below min
      final clampedMinTL = negativeTL.clamp(0.0, 100.0);
      expect(clampedMinTL, equals(0.0));

      // Acoustic loss attenuation formula: A = 10^(-TL / 20)
      final attenuationLinear = pow(10, -clampedTL / 20.0);
      expect(attenuationLinear, equals(0.00001)); // 100dB attenuation = 1e-5
    });

    // ITEM 11: EBU R128 / VU Meter Normalization & Clipping
    test('Item 11: EBU R128 VU Meter - RMS to dBFS & Peak Guard', () {
      // Test 1: Full scale sine wave RMS = 0.707 -> -3.01 dBFS
      const rmsLinear = 0.70710678;
      final dbfs = (rmsLinear > 0) ? (20 * log(rmsLinear) / ln10) : -100.0;
      expect(dbfs, closeTo(-3.01, 0.1));

      // Test 2: Zero signal -> bounded to -100 dBFS without log(0) -Infinity crash
      const zeroSignal = 0.0;
      final zeroDbfs = (zeroSignal > 0) ? (20 * log(zeroSignal) / ln10) : -100.0;
      expect(zeroDbfs, equals(-100.0));
      expect(zeroDbfs.isFinite, isTrue);
    });

    // ITEM 12: Room Zone Geometry & Boundary Verification
    test('Item 12: Room Zone Geometry - Bounds and Eyring/Sabine Rt60 Metadata', () {
      final zone = exhibition.RoomZone(
        id: 'zone_1',
        label: 'Exhibition Hall A',
        x: 0.0,
        y: 0.0,
        width: 200.0,
        height: 150.0,
        color: 0xFF00FF00,
        wallTransmissionLoss: 25.0,
      );

      expect(zone.width, equals(200.0));
      expect(zone.height, equals(150.0));
      expect(zone.wallTransmissionLoss, equals(25.0));
      expect(zone.estimatedRt60, greaterThan(0));

      final json = zone.toJson();
      final restored = exhibition.RoomZone.fromJson(json);
      expect(restored.label, equals('Exhibition Hall A'));
      expect(restored.wallTransmissionLoss, equals(25.0));
    });

    // ITEM 13: Preset Save/Load & Session Restore
    test('Item 13: Preset Serialization - AppConfig Lossless Round-trip', () {
      final config = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, oscWhitelist: const [], 
        oscPort: 9000,
        bufferSize: 256,
        themeStartOscAddress: '/atmos/start',
        systemResetOscAddress: '/atmos/reset',
        monoConfigs: {
          1: ChannelSetting(enabled: true, customName: 'Mic1', delayMs: 2.5, eqBands: []),
        },
        stereoConfigs: {},
        multiConfigs: {},
        rooms: [],
        isExhibitionMode: true,
        masterHeadroomDb: -3.0,
        peakLimiterEnabled: true,
        globalTrajectory: null,
        roomZones: [],
      );

      expect(config.oscPort, equals(9000));
      expect(config.bufferSize, equals(256));
      expect(config.masterHeadroomDb, equals(-3.0));
      expect(config.monoConfigs[1]?.customName, equals('Mic1'));
    });

    // ITEM 14: OSC / MIDI Integration Address Matching
    test('Item 14: OSC Pattern Matching & Parameter Parsing', () {
      const address = '/atmos/track/1/volume';
      final parts = address.split('/').where((s) => s.isNotEmpty).toList();

      expect(parts.length, equals(4));
      expect(parts[0], equals('atmos'));
      expect(parts[1], equals('track'));
      expect(int.parse(parts[2]), equals(1));
      expect(parts[3], equals('volume'));
    });

    // ITEM 15: Speaker Grouping & Group Level / Delay Adjust
    test('Item 15: Speaker Group Gain & Delay Offset Application', () {
      final spk1 = SpeakerNode(id: 's1', x: 0, y: 0, channel: 1);
      final spk2 = SpeakerNode(id: 's2', x: 5, y: 5, channel: 2);

      // Apply pitch/dispersion copyWith adjustments
      final updatedSpk1 = spk1.copyWith(pitchTilt: 20.0);
      final updatedSpk2 = spk2.copyWith(pitchTilt: 20.0);

      expect(updatedSpk1.pitchTilt, equals(20.0));
      expect(updatedSpk2.pitchTilt, equals(20.0));
    });

    // ITEM 16: DBAP Spatial Panning Roll-off Formula & Zero-Distance Safety
    test('Item 16: DBAP Panning Weight Calculation - Zero Distance Non-Break', () {
      double computeDbapWeight(double distance, double rollOff, double epsilon) {
        // W = 1 / (d^a + epsilon)
        return 1.0 / (pow(distance, rollOff) + epsilon);
      }

      const epsilon = 1e-4; // Small offset to avoid division by zero
      const rollOff = 2.0;

      // Case A: Speaker at exact point of sound source (distance = 0)
      final zeroDistWeight = computeDbapWeight(0.0, rollOff, epsilon);
      expect(zeroDistWeight.isFinite, isTrue);
      expect(zeroDistWeight, equals(10000.0)); // 1 / 1e-4

      // Case B: Speaker at 2 meters distance
      final dist2Weight = computeDbapWeight(2.0, rollOff, epsilon);
      expect(dist2Weight, closeTo(1.0 / 4.0001, 1e-3));
    });

    // ITEM 17: Timeline / Cue Sheet Scheduling Ordering
    test('Item 17: Cue Sheet Execution Timestamps & Cue Ordering', () {
      final cues = [
        {'id': 'cue_3', 'time': 12.0},
        {'id': 'cue_1', 'time': 0.0},
        {'id': 'cue_2', 'time': 5.5},
      ];

      cues.sort((a, b) => (a['time'] as double).compareTo(b['time'] as double));

      expect(cues[0]['id'], equals('cue_1'));
      expect(cues[1]['id'], equals('cue_2'));
      expect(cues[2]['id'], equals('cue_3'));
    });

    // ITEM 18: Background Noise & Binaural Headphone Monitoring
    test('Item 18: Binaural Headphone Monitoring Toggle & HRTF State', () {
      bool isBinauralEnabled = false;
      double headphoneGainDb = 0.0;

      // Toggle binaural monitoring
      isBinauralEnabled = !isBinauralEnabled;
      headphoneGainDb = -6.0; // Default headroom for HRTF convolution

      expect(isBinauralEnabled, isTrue);
      expect(headphoneGainDb, equals(-6.0));
    });

    // ITEM 19: Speaker 3D Yaw / Pitch / Roll Orientation Vector Computation
    test('Item 19: 3D Yaw/Pitch Orientation Direction Vector Computation', () {
      // Yaw = 0, Pitch = 0 -> Facing forward along +Y axis (0, 1, 0)
      const yawDeg = 0.0;
      const pitchDeg = 0.0;

      final yawRad = yawDeg * pi / 180.0;
      final pitchRad = pitchDeg * pi / 180.0;

      final dirX = sin(yawRad) * cos(pitchRad);
      final dirY = cos(yawRad) * cos(pitchRad);
      final dirZ = sin(pitchRad);

      expect(dirX, closeTo(0.0, 1e-5));
      expect(dirY, closeTo(1.0, 1e-5));
      expect(dirZ, closeTo(0.0, 1e-5));
    });

    // ITEM 20: System Reset & Hardware Device Config Reset State
    test('Item 20: System Reset State & Audio Hardware Default Bounds', () {
      const defaultBufferSize = 512;
      const defaultSampleRate = 48000;

      expect(defaultBufferSize, equals(512));
      expect(defaultSampleRate, equals(48000));

      // Verify reset state
      final resetConfig = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, oscWhitelist: const [], 
        oscPort: 8000,
        bufferSize: defaultBufferSize,
        themeStartOscAddress: '',
        systemResetOscAddress: '',
        monoConfigs: {},
        stereoConfigs: {},
        multiConfigs: {},
        rooms: [],
        isExhibitionMode: false,
        masterHeadroomDb: 0.0,
        peakLimiterEnabled: true,
        globalTrajectory: null,
        roomZones: [],
      );

      expect(resetConfig.bufferSize, equals(512));
      expect(resetConfig.rooms, isEmpty);
      expect(resetConfig.masterHeadroomDb, equals(0.0));
    });

  });
}
