// 통합 테스트 격리: 사용자의 실제 프로젝트(config.json, SharedPreferences)를 읽지도 쓰지도 않는다.
import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/spatial_reverb_state.dart';
import 'package:path_provider_platform_interface/path_provider_platform_interface.dart';
import 'package:plugin_platform_interface/plugin_platform_interface.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// 앱 지원 폴더 등을 테스트 임시 폴더로 돌린다.
class FakePathProvider extends PathProviderPlatform with MockPlatformInterfaceMixin {
  FakePathProvider(this.root);
  final String root;

  @override
  Future<String?> getApplicationSupportPath() async => root;
  @override
  Future<String?> getApplicationDocumentsPath() async => root;
  @override
  Future<String?> getApplicationCachePath() async => '$root/cache';
  @override
  Future<String?> getTemporaryPath() async => '$root/tmp';
  @override
  Future<String?> getLibraryPath() async => root;
  @override
  Future<String?> getDownloadsPath() async => '$root/downloads';
}

/// 톤 WAV 4개, config.json, 12m×12m 3D 방.
class Fixture {
  Fixture._(this.root, this.supportDir);
  final Directory root;
  final String supportDir;

  static const roomId = 'room_it';
  static const roomZoneId = 'room_it_zone';
  static const t200Loop = 't200_loop';
  static const t50Loop = 't50_loop';
  static const t200Long = 't200_long';
  static const t50Long = 't50_long';

  /// 화면 CH2 = 내부 채널 1(0부터 센다).
  static const mainChannel = 1;

  /// 화면 CH1 = 내부 채널 0.
  static const subChannel = 0;

  static Future<Fixture> create() async {
    final root = await Directory.systemTemp.createTemp('atmos_it_');
    final support = Directory('${root.path}/support')..createSync();
    final audio = Directory('${root.path}/audio')..createSync();
    String tone(String name, double hz, double seconds) {
      final path = '${audio.path}/$name.wav';
      writeToneWav(path, hz: hz, seconds: seconds);
      return path;
    }

    Map<String, Object?> track(String id, String path, {required bool loop, bool streaming = false}) => {
          'id': id,
          'name': id,
          'file_path': path,
          'volume': 1.0,
          'is_loop': loop,
          'is_streaming': streaming,
          'output_channel': mainChannel,
          'output_stereo': false,
        };

    final config = {
      'osc_port': 18000,
      // 시스템 기본 출력 장치(engine.rs start). 장치 이름을 하드코딩하지 않는다.
      'device_name': null,
      'buffer_size': 512,
      'rooms': [
        {
          'id': roomId,
          'name': 'IT Room',
          'color_hex': '#3B82F6',
          'volume': 1.0,
          'tracks': [
            track(t200Loop, tone('tone200_2s', 200, 2), loop: true),
            track(t50Loop, tone('tone50_2s', 50, 2), loop: true),
            // 5단계 재개가 두 경로를 다 지나게 한다: 200Hz는 스트리밍(DiskStreamer가 시작 위치까지
            // 건너뜀), 50Hz는 미리 로드(커서 이동). 루프 트랙도 스트리밍 경로다.
            track(t200Long, tone('tone200_20s', 200, 20), loop: false, streaming: true),
            track(t50Long, tone('tone50_20s', 50, 20), loop: false),
          ],
        },
      ],
    };
    File('${support.path}/config.json').writeAsStringSync(jsonEncode(config));
    return Fixture._(root, support.path);
  }

  /// 앱을 띄우기 전에 부른다. 경로·저장소를 가짜로 바꾸고 12m 방을 넣어 둔다.
  /// 방이 크면 스피커가 벽에서 멀어 200Hz에 경계면 보정이 거의 걸리지 않는다(3단계).
  void install() {
    PathProviderPlatform.instance = FakePathProvider(supportDir);
    const zone = RoomZone(
      id: roomZoneId,
      label: 'IT Zone',
      x: 0,
      y: 0,
      width: 12.0,
      height: 12.0,
      color: 0xFF0284C7,
      physicalWidth: 12.0,
      physicalHeight: 12.0,
      ceilingHeight: 4.0,
      earLevel: 1.2,
    );
    SharedPreferences.setMockInitialValues({
      'exhibition_room_zone_layout': jsonEncode([zone.toJson()]),
      // 공간 리버브 기본값(Hall, 3.2초, 80%)은 하드웨어 CH1·CH2 마스터 버스에 걸린다. 정지 뒤 잔향이
      // 몇 초 남고 CH2 소리를 CH1로 흘려, 무음·서브 라우팅 판정이 엔진 경로 대신 잔향을 재게 된다.
      'spatial_reverb_state': jsonEncode({
        'selectedChannel': 0,
        'channelSettings': {
          '0': const SpatialReverbSettings(isEnabled: false, dryWetPercent: 0).toJson(),
        },
      }),
    });
  }
}

/// 48kHz 모노 16비트 PCM 톤. 정수 주기로 끝나는 길이라 루프 이음매에 계단이 없다.
void writeToneWav(String path,
    {required double hz, required double seconds, double dbfs = -30, int sampleRate = 48000}) {
  final n = (seconds * sampleRate).round();
  final amp = pow(10, dbfs / 20) * 32767;
  final dataLen = n * 2;
  final b = ByteData(44 + dataLen);
  void ascii(int off, String s) {
    for (var i = 0; i < s.length; i++) {
      b.setUint8(off + i, s.codeUnitAt(i));
    }
  }

  ascii(0, 'RIFF');
  b.setUint32(4, 36 + dataLen, Endian.little);
  ascii(8, 'WAVE');
  ascii(12, 'fmt ');
  b.setUint32(16, 16, Endian.little);
  b.setUint16(20, 1, Endian.little); // PCM
  b.setUint16(22, 1, Endian.little); // 모노
  b.setUint32(24, sampleRate, Endian.little);
  b.setUint32(28, sampleRate * 2, Endian.little);
  b.setUint16(32, 2, Endian.little);
  b.setUint16(34, 16, Endian.little);
  ascii(36, 'data');
  b.setUint32(40, dataLen, Endian.little);
  for (var i = 0; i < n; i++) {
    b.setInt16(44 + i * 2, (amp * sin(2 * pi * hz * i / sampleRate)).round(), Endian.little);
  }
  File(path).writeAsBytesSync(b.buffer.asUint8List());
}
