// 다른 PC에서 저장한 프로젝트를 이 PC에서 열 때 트랙 오디오·도면 경로 다시 연결(실제 Rust 라이브러리).
// 화면은 띄우지 않고 프로젝트 열기가 부르는 함수만 실제로 부른다. 소리는 나지 않고, 실제 설정(config.json)과
// 환경설정은 건드리지 않는다. 실행: flutter test integration_test/project_media_relink_test.dart -d macos
import 'dart:io';
import 'dart:typed_data';

import 'package:atmos_mixer_pro/core/state/project_file.dart';
import 'package:atmos_mixer_pro/core/state/project_media.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:atmos_mixer_pro/src/rust/common/config.dart';
import 'package:atmos_mixer_pro/src/rust/frb_generated.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart' show ExternalLibrary;
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';

/// 0.1초짜리 모노 16비트 48kHz WAV(조용한 톤).
Uint8List _wav() {
  const rate = 48000, frames = 4800;
  final data = ByteData(44 + frames * 2);
  void ascii(int at, String s) {
    for (var i = 0; i < s.length; i++) {
      data.setUint8(at + i, s.codeUnitAt(i));
    }
  }

  ascii(0, 'RIFF');
  data.setUint32(4, 36 + frames * 2, Endian.little);
  ascii(8, 'WAVE');
  ascii(12, 'fmt ');
  data.setUint32(16, 16, Endian.little);
  data.setUint16(20, 1, Endian.little); // PCM
  data.setUint16(22, 1, Endian.little); // 모노
  data.setUint32(24, rate, Endian.little);
  data.setUint32(28, rate * 2, Endian.little);
  data.setUint16(32, 2, Endian.little);
  data.setUint16(34, 16, Endian.little);
  ascii(36, 'data');
  data.setUint32(40, frames * 2, Endian.little);
  for (var i = 0; i < frames; i++) {
    data.setInt16(44 + i * 2, (i % 48) < 24 ? 300 : -300, Endian.little);
  }
  return data.buffer.asUint8List();
}

TrackConfig _track(String id, String filePath) => TrackConfig(
      id: id,
      name: id,
      filePath: filePath,
      volume: 1.0,
      isLoop: false,
      isStreaming: false,
      outputChannel: 0,
      outputStereo: false,
      playOscAddress: '',
      stopOscAddress: '',
    );

AppConfig _config(List<TrackConfig> tracks) => AppConfig(
      oscPort: 8000,
      bufferSize: 256,
      themeStartOscAddress: '',
      systemResetOscAddress: '',
      monoConfigs: {},
      stereoConfigs: {},
      multiConfigs: {},
      rooms: [
        RoomConfig(
          id: 'r1',
          name: '방 1',
          colorHex: '#ffffff',
          volume: 1.0,
          volumeOscAddress: '',
          clearOscAddress: '',
          tracks: tracks,
        ),
      ],
      roomZones: [],
      isExhibitionMode: false,
      masterHeadroomDb: 0.0,
      peakLimiterEnabled: true,
      oscWhitelist: const [],
      globalReverbMix: 0.0,
      globalReverbDecay: 1.0,
    );

/// 디자이너 PC(macOS)에서 저장한 것처럼, 이 PC에 없는 경로가 든 프로젝트 파일을 만든다.
Future<String> _writeProject(Directory dir, List<TrackConfig> tracks, String blueprintPath) async {
  final path = '${dir.path}/project.atmos';
  await rust_api.apiSaveConfig(path: path, config: _config(tracks));
  final merged = injectExhibitionSection(await File(path).readAsString(), {'blueprint_image_path': blueprintPath});
  await File(path).writeAsString(merged);
  return path;
}

/// 대시보드의 프로젝트 열기(_loadProject)와 같은 순서로 다시 연결한다.
Future<ProjectMediaResult> _open(String path, Future<String?> Function(List<String>) askFolder) async {
  final imported = await rust_api.apiGetConfig(path: path);
  final rawBlueprint = readExhibitionSection(await File(path).readAsString())?['blueprint_image_path'];
  return relinkProjectMedia(
    config: imported,
    blueprintPath: rawBlueprint is String ? rawBlueprint : null,
    projectDir: File(path).parent.path,
    askFolder: askFolder,
  );
}

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(() async {
    await RustLib.init(
      externalLibrary: Platform.isMacOS
          ? ExternalLibrary.open('rust_lib_atmos_mixer_pro.framework/rust_lib_atmos_mixer_pro')
          : null,
    );
  });

  test('다른 PC에서 저장한 프로젝트를 열면 프로젝트 폴더의 파일로 다시 연결되고 엔진이 그 파일을 읽는다', () async {
    final root = await Directory.systemTemp.createTemp('atmos_relink_it_');
    addTearDown(() => root.delete(recursive: true));
    final project = await Directory('${root.path}/USB/Exhibit').create(recursive: true);
    final bgm = File('${project.path}/audio/room1/BGM.wav');
    await bgm.create(recursive: true);
    await bgm.writeAsBytes(_wav());
    final plan = File('${project.path}/plan/floor.png');
    await plan.create(recursive: true);
    await plan.writeAsBytes([0x89, 0x50, 0x4E, 0x47]);

    final path = await _writeProject(
      project,
      [_track('t1', '/Users/designer/Exhibit/audio/room1/bgm.wav')],
      r'C:\Shows\Exhibit\plan\floor.png',
    );
    final media = await _open(path, (_) async => fail('프로젝트 폴더에 다 있는데 폴더를 물었다'));
    expect(media.missing, isEmpty);
    expect(media.config.rooms.single.tracks.single.filePath, bgm.path);
    expect(media.blueprintPath, plan.path);
    // 다시 연결한 경로로 엔진이 오디오를 미리 읽는다(예전에는 여기서 조용히 실패했다)
    await rust_api.apiPreloadSound(filePath: media.config.rooms.single.tracks.single.filePath);
  });

  test('프로젝트 폴더에 없는 파일은 고른 폴더에서 찾고, 끝까지 없는 파일은 목록으로 남는다', () async {
    final root = await Directory.systemTemp.createTemp('atmos_relink_it_');
    addTearDown(() => root.delete(recursive: true));
    final project = await Directory('${root.path}/project').create(recursive: true);
    final other = File('${root.path}/audio_on_d/sfx/door.wav');
    await other.create(recursive: true);
    await other.writeAsBytes(_wav());

    final path = await _writeProject(
      project,
      [_track('t1', '/Users/designer/Exhibit/sfx/door.wav'), _track('t2', '/Users/designer/Exhibit/gone.wav')],
      '/Users/designer/Exhibit/plan/floor.png',
    );
    List<String>? asked;
    final media = await _open(path, (missing) async {
      asked = missing;
      return '${root.path}/audio_on_d';
    });
    expect(asked, [
      '/Users/designer/Exhibit/sfx/door.wav',
      '/Users/designer/Exhibit/gone.wav',
      '/Users/designer/Exhibit/plan/floor.png',
    ]);
    final tracks = media.config.rooms.single.tracks;
    expect(tracks[0].filePath, other.path);
    expect(tracks[1].filePath, '/Users/designer/Exhibit/gone.wav', reason: '못 찾은 트랙은 원래 경로를 둔다');
    expect(media.missing, ['/Users/designer/Exhibit/gone.wav', '/Users/designer/Exhibit/plan/floor.png']);
    expect(media.blueprintPath, '/Users/designer/Exhibit/plan/floor.png');
  });
}
