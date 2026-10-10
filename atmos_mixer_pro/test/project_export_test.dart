import 'dart:convert';
import 'dart:io';

import 'package:atmos_mixer_pro/core/state/project_export.dart';
import 'package:atmos_mixer_pro/core/state/project_file.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';
import 'package:flutter_test/flutter_test.dart';

/// 프로젝트 내보내기(사용자 요청 2026-10-10): 정한 이름의 폴더에 project.atmos + audio/ + drawing/을 모은다.
void main() {
  TrackConfig track(String id, String path) => TrackConfig(
        id: id,
        name: id,
        filePath: path,
        volume: 0.5,
        isLoop: false,
        isStreaming: false,
        outputChannel: 0,
        outputStereo: true,
        playOscAddress: '',
        stopOscAddress: '',
      );
  AppConfig config(List<String> paths) => AppConfig(
        oscPort: 8000,
        deviceName: '[ASIO] ASIO MADIface USB',
        bufferSize: 1024,
        themeStartOscAddress: '',
        systemResetOscAddress: '',
        monoConfigs: const {},
        stereoConfigs: const {},
        multiConfigs: const {},
        rooms: [
          RoomConfig(
            id: 'r1',
            name: '테마 1',
            colorHex: 'FF0000',
            volume: 1.0,
            volumeOscAddress: '',
            clearOscAddress: '',
            tracks: [for (var i = 0; i < paths.length; i++) track('t$i', paths[i])],
          ),
        ],
        roomZones: const [],
        isExhibitionMode: true,
        masterHeadroomDb: 0.0,
        peakLimiterEnabled: true,
        oscWhitelist: const [],
        globalReverbMix: 0.0,
        globalReverbDecay: 1.0,
      );

  group('복사 계획', () {
    test('같은 원본은 한 번만, 이름이 같은 다른 원본은 (2)를 붙인다(대소문자 무시)', () {
      final plan = planProjectExport(trackSources: [
        r'C:\a\loop.wav',
        '/Users/me/b/LOOP.wav',
        r'C:\a\loop.wav',
        '/Users/me/b/raf beat(Bbm 143 @prodallweno).mp3',
      ], blueprintSource: '/Users/me/plan.png');
      expect(plan.map((c) => c.relativeDest), [
        'audio/loop.wav',
        'audio/LOOP (2).wav',
        'audio/raf beat(Bbm 143 @prodallweno).mp3',
        'drawing/plan.png',
      ]);
    });

    test('확장자가 없는 이름도 번호를 붙인다', () {
      final plan = planProjectExport(trackSources: ['/x/noext', '/y/noext']);
      expect(plan.map((c) => c.relativeDest), ['audio/noext', 'audio/noext (2)']);
    });
  });

  test('폴더 이름 검사', () {
    expect(validateExportFolderName('전시 2026 가을'), isNull);
    expect(validateExportFolderName('  '), isNotNull);
    expect(validateExportFolderName('a/b'), isNotNull);
    expect(validateExportFolderName('what?'), isNotNull);
    expect(validateExportFolderName('끝에 점.'), isNotNull);
    expect(validateExportFolderName('con'), isNotNull);
    expect(validateExportFolderName('COM1.backup'), isNotNull);
    expect(validateExportFolderName('console'), isNull);
  });

  group('내보내기', () {
    late Directory root;
    setUp(() async => root = await Directory.systemTemp.createTemp('atmos_export_test_'));
    tearDown(() async => root.delete(recursive: true));

    Future<String> write(String relative, String content) async {
      final f = File('${root.path}${Platform.pathSeparator}${relative.replaceAll('/', Platform.pathSeparator)}');
      await f.parent.create(recursive: true);
      await f.writeAsString(content);
      return f.path;
    }

    test('음원·도면을 모으고 경로를 복사본으로 바꿔 저장한다. 원본이 없는 파일은 원래 경로로 남긴다', () async {
      final a = await write('src/a/loop.wav', 'A');
      final b = await write('src/b/loop.wav', 'B');
      final c = await write('src/c/beat.mp3', 'C');
      final blueprint = await write('src/plan.png', 'P');
      const gone = '/Users/Allweno/Downloads/missing.wav';
      AppConfig? saved;
      final progress = <String>[];

      final result = await exportProject(
        parentDir: '${root.path}${Platform.pathSeparator}out',
        folderName: '전시 내보내기',
        config: config([a, b, c, gone, c]),
        design: {'blueprint_image_path': blueprint, 'blueprint_opacity': 0.5, 'tuning_state': '{}'},
        saveConfig: (path, cfg) async {
          saved = cfg;
          await File(path).writeAsString(jsonEncode({'osc_port': cfg.oscPort}));
        },
        onProgress: (done, total, name) => progress.add('$done/$total'),
      );

      final sep = Platform.pathSeparator;
      final folder = result.folder;
      expect(folder, '${root.path}${sep}out$sep전시 내보내기');
      expect(await File('$folder${sep}audio${sep}loop.wav').readAsString(), 'A');
      expect(await File('$folder${sep}audio${sep}loop (2).wav').readAsString(), 'B');
      expect(await File('$folder${sep}audio${sep}beat.mp3').readAsString(), 'C');
      expect(await File('$folder${sep}drawing${sep}plan.png').readAsString(), 'P');
      expect(result.copied, 4);
      expect(result.missing, [gone]);
      expect(progress.last, '5/5');

      final paths = saved!.rooms.single.tracks.map((t) => t.filePath).toList();
      expect(paths, [
        '$folder${sep}audio${sep}loop.wav',
        '$folder${sep}audio${sep}loop (2).wav',
        '$folder${sep}audio${sep}beat.mp3',
        gone,
        '$folder${sep}audio${sep}beat.mp3',
      ]);
      expect(saved!.rooms.single.tracks.first.volume, 0.5, reason: '경로 말고는 그대로다');
      expect(saved!.deviceName, '[ASIO] ASIO MADIface USB');

      final design = readExhibitionSection(await File(result.projectPath).readAsString())!;
      expect(design['blueprint_image_path'], '$folder${sep}drawing${sep}plan.png');
      expect(design['blueprint_opacity'], 0.5);
      expect(design['tuning_state'], '{}');
    });

    test('같은 폴더로 다시 내보내도 원본을 지우지 않는다', () async {
      final sep = Platform.pathSeparator;
      final inPlace = await write('show/audio/loop.wav', 'ORIGINAL');
      final result = await exportProject(
        parentDir: root.path,
        folderName: 'show',
        config: config([inPlace]),
        design: const {},
        saveConfig: (path, cfg) async => File(path).writeAsString('{}'),
      );
      expect(await File('${result.folder}${sep}audio${sep}loop.wav').readAsString(), 'ORIGINAL');
      expect(result.copied, 1);
    });
  });
}
