import 'package:atmos_mixer_pro/core/state/config_copy.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';
import 'package:flutter_test/flutter_test.dart';

/// 설정 복사(core/state/config_copy.dart, 사용자 결정 2026-10-11). 예전에는 화면 33곳이 설정을 칸마다 다시 적으며
/// 전역 리버브를 0%·1.0으로, 방 카드는 OSC 허용 목록을 빈 목록으로 덮고 머리 추적 OSC 주소를 빠뜨렸다.
/// 칸을 하나 더하면 [full]에도 기본값이 아닌 값을 넣어야 이 테스트가 그 칸을 지킨다.
void main() {
  const point = Point3D(x: 1, y: 2, z: 3, yawRotation: 4, pitchTilt: 5, dispersionAngle: 6, size: 7);

  /// 모든 칸이 기본값이 아니고, 목록·맵은 새로 만든 것(같은 것인지로 비교된다).
  AppConfig full() => AppConfig(
        oscPort: 9123,
        deviceName: '[ASIO] ASIO MADIface USB',
        bufferSize: 512,
        themeStartOscAddress: '/theme/start',
        systemResetOscAddress: '/system/reset',
        trackingOscAddress: '/hrtf/custom',
        monoConfigs: <int, ChannelSetting>{},
        stereoConfigs: <int, ChannelSetting>{},
        multiConfigs: <int, ChannelSetting>{},
        rooms: <RoomConfig>[
          const RoomConfig(
            id: 'r1',
            name: '테마 1',
            colorHex: '#123456',
            volume: 0.5,
            volumeOscAddress: '',
            clearOscAddress: '/room1/clear',
            tracks: [],
          ),
        ],
        globalTrajectory: const Trajectory(waypoints: [point], currentPosition: point, targetRoomZoneId: 'r1'),
        roomZones: <RoomZone>[],
        isExhibitionMode: true,
        masterHeadroomDb: -3.0,
        peakLimiterEnabled: false,
        oscWhitelist: <String>['10.0.0.7'],
        globalReverbMix: 0.4,
        globalReverbDecay: 2.5,
      );

  test('아무것도 넘기지 않으면 모든 칸이 그대로다', () {
    final config = full();
    expect(config.copyWith(), config);
  });

  test('넘긴 칸만 바뀌고 리버브·허용 목록·추적 주소는 그대로다', () {
    final config = full();
    final changed = config.copyWith(rooms: <RoomConfig>[], oscPort: 9001);
    expect(changed.oscPort, 9001);
    expect(changed.rooms, isEmpty);
    expect(changed.globalReverbMix, 0.4);
    expect(changed.globalReverbDecay, 2.5);
    expect(changed.oscWhitelist, ['10.0.0.7']);
    expect(changed.trackingOscAddress, '/hrtf/custom');
    expect(changed.copyWith(rooms: config.rooms, oscPort: config.oscPort), config);
  });

  test('비어 있을 수 있는 칸은 넘기지 않으면 그대로, null을 넘기면 비운다', () {
    final config = full();
    expect(config.copyWith(deviceName: null).deviceName, isNull);
    expect(config.copyWith(trackingOscAddress: null).trackingOscAddress, isNull);
    expect(config.copyWith(globalTrajectory: null).globalTrajectory, isNull);
    expect(config.copyWith(deviceName: '[WASAPI] Analog (3+4)').deviceName, '[WASAPI] Analog (3+4)');
    expect(config.copyWith(bufferSize: 1024).deviceName, config.deviceName);
  });
}
