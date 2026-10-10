import 'package:atmos_mixer_pro/core/state/project_media.dart';
import 'package:atmos_mixer_pro/src/rust/api/project.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';
import 'package:flutter_test/flutter_test.dart';

AppConfig _config(String tag) => AppConfig(
      oscPort: 8000,
      bufferSize: 256,
      themeStartOscAddress: tag,
      systemResetOscAddress: '',
      monoConfigs: {},
      stereoConfigs: {},
      multiConfigs: {},
      rooms: [],
      roomZones: [],
      isExhibitionMode: false,
      masterHeadroomDb: 0.0,
      peakLimiterEnabled: true,
      oscWhitelist: const [],
      globalReverbMix: 0.0,
      globalReverbDecay: 1.0,
    );

/// 가짜 Rust 다시 연결: 폴더마다 찾을 수 있는 원래 경로를 정해 둔다.
class _FakeDisk {
  _FakeDisk.withTracks(this.findable, this.pending);

  /// 폴더 → 그 폴더에서 찾을 수 있는 원래 경로 → 새 경로
  final Map<String, Map<String, String>> findable;
  final List<List<String>> trackCalls = [];

  /// 아직 못 찾은 트랙 경로(첫 호출 전에는 전부)
  List<String> pending;

  Future<RelinkedConfig> relinkTracks(AppConfig config, List<String> dirs) async {
    trackCalls.add(dirs);
    final found = <String>{};
    for (final dir in dirs) {
      found.addAll(pending.where((p) => findable[dir]?.containsKey(p) ?? false));
    }
    pending = pending.where((p) => !found.contains(p)).toList();
    return RelinkedConfig(config: _config('relinked-${trackCalls.length}'), missing: pending);
  }

  Future<String?> findMedia(String path, List<String> dirs) async {
    for (final dir in dirs) {
      final hit = findable[dir]?[path];
      if (hit != null) return hit;
    }
    return null;
  }
}

void main() {
  group('프로젝트 파일 다시 연결 순서', () {
    test('모두 프로젝트 폴더에서 찾으면 폴더를 묻지 않는다', () async {
      final disk = _FakeDisk.withTracks({
        '/proj': {'/mac/a.wav': '/proj/a.wav', '/mac/plan.png': '/proj/plan.png'},
      }, ['/mac/a.wav']);
      var asked = false;
      final result = await relinkProjectMedia(
        config: _config('original'),
        blueprintPath: '/mac/plan.png',
        projectDir: '/proj',
        askFolder: (_) async {
          asked = true;
          return null;
        },
        relinkTracks: disk.relinkTracks,
        findMedia: disk.findMedia,
      );
      expect(asked, isFalse);
      expect(disk.trackCalls, [
        ['/proj'],
      ]);
      expect(result.missing, isEmpty);
      expect(result.blueprintPath, '/proj/plan.png');
      expect(result.config.themeStartOscAddress, 'relinked-1', reason: '다시 연결한 엔진 설정을 써야 한다');
    });

    test('남은 파일이 있으면 트랙과 도면을 함께 보여 주며 폴더를 묻고 그 폴더에서 다시 찾는다', () async {
      final disk = _FakeDisk.withTracks({
        '/proj': {'/mac/a.wav': '/proj/a.wav'},
        '/usb/audio': {'/mac/b.wav': '/usb/audio/b.wav', '/mac/plan.png': '/usb/audio/plan.png'},
      }, ['/mac/a.wav', '/mac/b.wav', '/mac/c.wav']);
      List<String>? askedWith;
      final result = await relinkProjectMedia(
        config: _config('original'),
        blueprintPath: '/mac/plan.png',
        projectDir: '/proj',
        askFolder: (missing) async {
          askedWith = missing;
          return '/usb/audio';
        },
        relinkTracks: disk.relinkTracks,
        findMedia: disk.findMedia,
      );
      expect(askedWith, ['/mac/b.wav', '/mac/c.wav', '/mac/plan.png']);
      expect(disk.trackCalls, [
        ['/proj'],
        ['/usb/audio'],
      ]);
      expect(result.missing, ['/mac/c.wav'], reason: '끝까지 못 찾은 것만 남는다');
      expect(result.blueprintPath, '/usb/audio/plan.png');
      expect(result.config.themeStartOscAddress, 'relinked-2');
    });

    test('폴더를 고르지 않으면 그대로 열고 못 찾은 목록을 돌려준다', () async {
      final disk = _FakeDisk.withTracks({'/proj': {}}, ['/mac/a.wav']);
      final result = await relinkProjectMedia(
        config: _config('original'),
        blueprintPath: '/mac/plan.png',
        projectDir: '/proj',
        askFolder: (_) async => null,
        relinkTracks: disk.relinkTracks,
        findMedia: disk.findMedia,
      );
      expect(disk.trackCalls.length, 1);
      expect(result.missing, ['/mac/a.wav', '/mac/plan.png']);
      expect(result.blueprintPath, '/mac/plan.png', reason: '못 찾은 도면은 원래 경로를 둔다');
    });

    test('도면이 없는 프로젝트는 도면을 찾지 않는다', () async {
      final disk = _FakeDisk.withTracks({'/proj': {}}, []);
      final result = await relinkProjectMedia(
        config: _config('original'),
        blueprintPath: null,
        projectDir: '/proj',
        askFolder: (_) async => fail('찾을 것이 없는데 폴더를 물었다'),
        relinkTracks: disk.relinkTracks,
        findMedia: (path, dirs) async => fail('도면이 없는데 찾았다'),
      );
      expect(result.missing, isEmpty);
      expect(result.blueprintPath, isNull);
    });
  });

  // 2026-10-10 Windows P6-E: 맥에서 저장한 프로젝트를 열자 엔진이 맥 장치를 30초 동안 찾으며 ASIO 드라이버를
  // 반복해서 불러오고(Generic Low Latency ASIO 창 반복) 끝내 오류로 멈췄다. 이 PC에 없는 장치면 지금 장치를 쓴다.
  group('프로젝트를 열 때 오디오 장치', () {
    AppConfig cfg({String? device, required int buffer, required String tag}) => AppConfig(
          oscPort: 8000,
          deviceName: device,
          bufferSize: buffer,
          themeStartOscAddress: tag,
          systemResetOscAddress: '',
          monoConfigs: {},
          stereoConfigs: {},
          multiConfigs: {},
          rooms: [],
          roomZones: [],
          isExhibitionMode: false,
          masterHeadroomDb: 0.0,
          peakLimiterEnabled: true,
          oscWhitelist: const [],
          globalReverbMix: 0.0,
          globalReverbDecay: 1.0,
        );
    final current = cfg(device: '[ASIO] ASIO MADIface USB', buffer: 1024, tag: 'current');
    const windowsDevices = ['[ASIO] ASIO MADIface USB', '[WASAPI] Analog (1+2) (RME UFX+ USB 3.0)'];

    test('이 PC에 없는 장치(맥 장치)면 지금 장치와 버퍼를 쓰고 나머지는 프로젝트 값이다', () {
      final mac = cfg(device: '[CoreAudio] Scarlett 6i6 USB', buffer: 256, tag: 'project');
      final choice = deviceForImportedProject(imported: mac, current: current, available: windowsDevices);
      expect(choice.keptCurrent, isTrue);
      expect(choice.projectDevice, '[CoreAudio] Scarlett 6i6 USB');
      expect(choice.config.deviceName, '[ASIO] ASIO MADIface USB');
      expect(choice.config.bufferSize, 1024);
      expect(choice.config.themeStartOscAddress, 'project', reason: '장치 말고는 프로젝트 설정을 그대로 쓴다');
    });

    test('이 PC에 있는 장치면 프로젝트의 장치와 버퍼를 쓴다', () {
      final wasapi = cfg(device: '[WASAPI] Analog (1+2) (RME UFX+ USB 3.0)', buffer: 512, tag: 'project');
      final choice = deviceForImportedProject(imported: wasapi, current: current, available: windowsDevices);
      expect(choice.keptCurrent, isFalse);
      expect(identical(choice.config, wasapi), isTrue);
    });

    test('지금 쓰는 장치와 같으면 목록에 없어도 그대로 쓴다', () {
      final same = cfg(device: ' [ASIO] ASIO MADIface USB', buffer: 2048, tag: 'project');
      final choice = deviceForImportedProject(imported: same, current: current, available: const []);
      expect(choice.keptCurrent, isFalse);
      expect(choice.config.bufferSize, 2048);
    });

    test('기본 장치(null)를 쓰는 프로젝트나 장치 목록을 모를 때는 프로젝트 값을 그대로 쓴다', () {
      final defaultDevice = cfg(device: null, buffer: 256, tag: 'project');
      expect(deviceForImportedProject(imported: defaultDevice, current: current, available: windowsDevices).keptCurrent,
          isFalse);
      final mac = cfg(device: '[CoreAudio] Scarlett 6i6 USB', buffer: 256, tag: 'project');
      final unknown = deviceForImportedProject(imported: mac, current: current, available: null);
      expect(unknown.keptCurrent, isFalse);
      expect(unknown.config.deviceName, '[CoreAudio] Scarlett 6i6 USB');
      expect(deviceForImportedProject(imported: mac, current: null, available: windowsDevices).keptCurrent, isFalse);
    });
  });

  // 프로젝트가 지금 장치보다 많은 채널을 쓰면 장치 안내 창에 함께 알린다(2026-10-10 사용자 요청).
  group('프로젝트가 쓰는 채널 수', () {
    ChannelSetting on() => const ChannelSetting(
        enabled: true, customName: '', delayMs: 0.0, eqBands: [], phaseInvert: false, gainDb: 0.0);
    TrackConfig track(int outputChannel, {required bool stereo}) => TrackConfig(
          id: 't$outputChannel',
          name: 't',
          filePath: '/x.wav',
          volume: 1.0,
          isLoop: false,
          isStreaming: false,
          outputChannel: outputChannel,
          outputStereo: stereo,
          playOscAddress: '',
          stopOscAddress: '',
        );
    AppConfig project({
      Map<int, ChannelSetting> mono = const {},
      Map<int, ChannelSetting> stereo = const {},
      Map<int, ChannelSetting> multi = const {},
      List<TrackConfig> tracks = const [],
    }) =>
        AppConfig(
          oscPort: 8000,
          bufferSize: 1024,
          themeStartOscAddress: '',
          systemResetOscAddress: '',
          monoConfigs: mono,
          stereoConfigs: stereo,
          multiConfigs: multi,
          rooms: [
            RoomConfig(
              id: 'r',
              name: 'r',
              colorHex: 'FF0000',
              volume: 1.0,
              volumeOscAddress: '',
              clearOscAddress: '',
              tracks: tracks,
            ),
          ],
          roomZones: const [],
          isExhibitionMode: false,
          masterHeadroomDb: 0.0,
          peakLimiterEnabled: true,
          oscWhitelist: const [],
          globalReverbMix: 0.0,
          globalReverbDecay: 1.0,
        );

    test('맥 프로젝트처럼 CH12까지 켰으면 12, 자리표 255는 세지 않는다', () {
      final mac = project(
        mono: {for (var k = 1; k <= 12; k++) k: on(), 255: on()},
        stereo: {for (var k = 1; k <= 11; k += 2) k: on(), 255: on()},
        multi: {1: on()},
        tracks: [track(0, stereo: true), track(1, stereo: false)],
      );
      expect(highestProjectChannel(mac), 12);
    });

    test('트랙 출력도 센다(스테레오는 다음 채널까지)', () {
      expect(highestProjectChannel(project(tracks: [track(6, stereo: true)])), 8);
      expect(highestProjectChannel(project(tracks: [track(6, stereo: false)])), 7);
    });

    test('장치가 모자랄 때만 안내한다', () {
      expect(channelShortfallNotice(projectHighest: 12, deviceChannels: 2),
          '프로젝트는 CH12까지 쓰는데 지금 장치는 2채널이라 CH3~CH12는 소리가 나지 않습니다.');
      expect(channelShortfallNotice(projectHighest: 12, deviceChannels: 94), isNull);
      expect(channelShortfallNotice(projectHighest: 12, deviceChannels: 0), isNull, reason: '장치 채널 수를 모르면 말하지 않는다');
    });
  });
}
