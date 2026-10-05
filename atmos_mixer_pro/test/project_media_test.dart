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
}
