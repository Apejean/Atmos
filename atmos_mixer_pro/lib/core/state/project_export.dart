import 'dart:io';

import 'package:atmos_mixer_pro/core/state/project_file.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';

/// 프로젝트 내보내기(File > Export Project, 사용자 요청 2026-10-10).
///
/// 사용자가 정한 이름의 폴더 하나에 `project.atmos`(엔진 설정 + 설계 데이터)와 프로젝트가 쓰는 음원
/// (`audio/`), 도면 이미지(`drawing/`)를 모은다. 다른 PC에서는 그 폴더를 통째로 옮겨 Load Project로 연다.
/// 열 때 저장된 경로에 파일이 없으면 `.atmos` 폴더에서 같은 이름으로 다시 연결한다(project_media.dart).

/// 복사 한 건. [relativeDest]는 내보내는 폴더 기준 경로이고 구분자는 `/`다.
class ExportCopy {
  const ExportCopy(this.source, this.relativeDest);

  final String source;
  final String relativeDest;
}

/// 복사 계획. 같은 원본은 한 번만 복사하고, 이름이 같은 다른 원본은 `이름 (2).wav`처럼 바꾼다(대소문자 무시).
List<ExportCopy> planProjectExport({required List<String> trackSources, String? blueprintSource}) {
  final copies = <ExportCopy>[];
  final seen = <String>{};
  final used = <String>{};

  String unique(String folder, String name) {
    final dot = name.lastIndexOf('.');
    final stem = dot > 0 ? name.substring(0, dot) : name;
    final ext = dot > 0 ? name.substring(dot) : '';
    var candidate = name;
    var n = 2;
    while (!used.add('$folder/$candidate'.toLowerCase())) {
      candidate = '$stem ($n)$ext';
      n++;
    }
    return '$folder/$candidate';
  }

  for (final source in trackSources) {
    if (source.isEmpty || !seen.add(source)) continue;
    copies.add(ExportCopy(source, unique('audio', fileNameOf(source))));
  }
  if (blueprintSource != null && blueprintSource.isNotEmpty) {
    copies.add(ExportCopy(blueprintSource, unique('drawing', fileNameOf(blueprintSource))));
  }
  return copies;
}

/// 경로의 마지막 이름. 다른 OS에서 저장한 경로도 있으니 `/`와 `\`를 모두 구분자로 본다.
String fileNameOf(String path) {
  final i = path.lastIndexOf(RegExp(r'[/\\]'));
  return i < 0 ? path : path.substring(i + 1);
}

/// 폴더 이름이 쓸 수 없으면 이유를, 쓸 수 있으면 null을 돌려준다. 맥에서 만든 폴더를 Windows로 옮겨도
/// 열리도록 Windows 규칙(금지 문자, 끝의 점·공백, 예약 이름)을 따른다.
String? validateExportFolderName(String name) {
  final trimmed = name.trim();
  if (trimmed.isEmpty) return '폴더 이름을 입력하세요.';
  if (trimmed == '.' || trimmed == '..') return '쓸 수 없는 폴더 이름입니다.';
  if (RegExp(r'[<>:"/\\|?*\x00-\x1F]').hasMatch(trimmed)) {
    return r'폴더 이름에 쓸 수 없는 문자가 있습니다(< > : " / \ | ? *).';
  }
  if (name.endsWith('.') || name.endsWith(' ')) return '폴더 이름은 점이나 공백으로 끝날 수 없습니다.';
  final base = trimmed.split('.').first.toUpperCase();
  if (RegExp(r'^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])$').hasMatch(base)) {
    return 'Windows가 예약한 이름이라 쓸 수 없습니다.';
  }
  return null;
}

/// 내보내기 결과.
class ProjectExportResult {
  const ProjectExportResult({
    required this.folder,
    required this.projectPath,
    required this.copied,
    required this.missing,
  });

  /// 내보낸 폴더.
  final String folder;

  /// 쓴 프로젝트 파일.
  final String projectPath;

  /// 복사한(또는 이미 그 자리에 있던) 파일 수.
  final int copied;

  /// 원본이 없어 복사하지 못한 파일. 프로젝트에는 원래 경로가 남는다.
  final List<String> missing;
}

/// 엔진이 설정을 프로젝트 파일로 쓴다(Rust `api_save_config`).
typedef SaveProjectConfig = Future<void> Function(String path, AppConfig config);

/// [parentDir] 아래 [folderName] 폴더에 프로젝트를 내보낸다. 트랙·도면 경로는 복사본을 가리키게 바꿔 저장한다.
/// [design]은 설계 데이터(project_file.dart `collectExhibitionData`)다. 원본이 없는 파일은 건너뛰고
/// [ProjectExportResult.missing]에 남긴다. 복사 중 오류(디스크 부족·권한)는 그대로 던진다.
Future<ProjectExportResult> exportProject({
  required String parentDir,
  required String folderName,
  required AppConfig config,
  required Map<String, dynamic> design,
  required SaveProjectConfig saveConfig,
  void Function(int done, int total, String name)? onProgress,
}) async {
  final folder = _join(parentDir, folderName.trim());
  await Directory(folder).create(recursive: true);

  final blueprint = design['blueprint_image_path'];
  final blueprintSource = blueprint is String && blueprint.isNotEmpty ? blueprint : null;
  final copies = planProjectExport(
    trackSources: [for (final room in config.rooms) for (final track in room.tracks) track.filePath],
    blueprintSource: blueprintSource,
  );

  final newPaths = <String, String>{};
  final missing = <String>[];
  for (var i = 0; i < copies.length; i++) {
    final copy = copies[i];
    onProgress?.call(i, copies.length, fileNameOf(copy.source));
    final source = File(copy.source);
    if (!await source.exists()) {
      missing.add(copy.source);
      continue;
    }
    final dest = _join(folder, copy.relativeDest);
    await Directory(File(dest).parent.path).create(recursive: true);
    // 원본이 이미 그 자리에 있으면(같은 폴더로 다시 내보내기) 복사하지 않는다. 자기 자신 위에 복사하면
    // 내용이 지워질 수 있다.
    if (!_samePath(source.absolute.path, dest)) {
      await source.copy(dest);
    }
    newPaths[copy.source] = dest;
  }
  onProgress?.call(copies.length, copies.length, '');

  final exportedDesign = Map<String, dynamic>.from(design);
  if (blueprintSource != null && newPaths.containsKey(blueprintSource)) {
    exportedDesign['blueprint_image_path'] = newPaths[blueprintSource];
  }
  final projectPath = _join(folder, 'project.atmos');
  await saveConfig(projectPath, _withTrackPaths(config, newPaths));
  final file = File(projectPath);
  await file.writeAsString(injectExhibitionSection(await file.readAsString(), exportedDesign));

  return ProjectExportResult(
    folder: folder,
    projectPath: projectPath,
    copied: newPaths.length,
    missing: missing,
  );
}

String _join(String dir, String relative) {
  final sep = Platform.pathSeparator;
  final rel = relative.replaceAll('/', sep);
  return dir.endsWith(sep) || dir.endsWith('/') ? '$dir$rel' : '$dir$sep$rel';
}

bool _samePath(String a, String b) {
  String norm(String p) => File(p).absolute.path.replaceAll('\\', '/').toLowerCase();
  return norm(a) == norm(b);
}

/// 트랙 경로만 바꾼 설정. 생성된 설정 타입에 copyWith가 없어 필드를 모두 넘긴다.
AppConfig _withTrackPaths(AppConfig c, Map<String, String> newPaths) => AppConfig(
      oscPort: c.oscPort,
      deviceName: c.deviceName,
      bufferSize: c.bufferSize,
      themeStartOscAddress: c.themeStartOscAddress,
      systemResetOscAddress: c.systemResetOscAddress,
      trackingOscAddress: c.trackingOscAddress,
      monoConfigs: c.monoConfigs,
      stereoConfigs: c.stereoConfigs,
      multiConfigs: c.multiConfigs,
      rooms: [
        for (final r in c.rooms)
          RoomConfig(
            id: r.id,
            name: r.name,
            colorHex: r.colorHex,
            volume: r.volume,
            volumeOscAddress: r.volumeOscAddress,
            clearOscAddress: r.clearOscAddress,
            tracks: [
              for (final t in r.tracks)
                TrackConfig(
                  id: t.id,
                  name: t.name,
                  filePath: newPaths[t.filePath] ?? t.filePath,
                  volume: t.volume,
                  isLoop: t.isLoop,
                  isStreaming: t.isStreaming,
                  outputChannel: t.outputChannel,
                  outputStereo: t.outputStereo,
                  playOscAddress: t.playOscAddress,
                  stopOscAddress: t.stopOscAddress,
                ),
            ],
          ),
      ],
      globalTrajectory: c.globalTrajectory,
      roomZones: c.roomZones,
      isExhibitionMode: c.isExhibitionMode,
      masterHeadroomDb: c.masterHeadroomDb,
      peakLimiterEnabled: c.peakLimiterEnabled,
      oscWhitelist: c.oscWhitelist,
      globalReverbMix: c.globalReverbMix,
      globalReverbDecay: c.globalReverbDecay,
    );
