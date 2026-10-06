import 'package:atmos_mixer_pro/src/rust/api/project.dart' as project_api;
import 'package:atmos_mixer_pro/src/rust/common/config.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// 프로젝트를 열 때 다시 연결한 결과.
class ProjectMediaResult {
  const ProjectMediaResult({required this.config, required this.blueprintPath, required this.missing});

  /// 트랙 오디오 경로를 다시 연결한 엔진 설정.
  final AppConfig config;

  /// 도면 이미지 경로(다시 연결했으면 새 경로, 못 찾았으면 원래 경로). 프로젝트에 도면이 없으면 null.
  final String? blueprintPath;

  /// 끝까지 못 찾은 파일의 원래 경로.
  final List<String> missing;
}

typedef RelinkTracks = Future<project_api.RelinkedConfig> Function(AppConfig config, List<String> searchDirs);
typedef FindMedia = Future<String?> Function(String path, List<String> searchDirs);

Future<project_api.RelinkedConfig> _relinkTracks(AppConfig config, List<String> searchDirs) =>
    project_api.apiRelinkTrackPaths(config: config, searchDirs: searchDirs);

Future<String?> _findMedia(String path, List<String> searchDirs) =>
    project_api.apiFindMedia(path: path, searchDirs: searchDirs);

/// 프로젝트 파일을 다른 PC에서 열 때 트랙 오디오·도면 경로를 다시 연결한다(rust core::media_relink).
/// 먼저 프로젝트 파일이 있는 폴더(하위 폴더 포함)에서 파일 이름으로 찾는다. 그래도 못 찾은 파일이 있으면
/// [askFolder]로 폴더를 물어 거기서 다시 찾고, 고르지 않으면(null) 그대로 둔다.
Future<ProjectMediaResult> relinkProjectMedia({
  required AppConfig config,
  required String? blueprintPath,
  required String projectDir,
  required Future<String?> Function(List<String> missing) askFolder,
  RelinkTracks relinkTracks = _relinkTracks,
  FindMedia findMedia = _findMedia,
}) async {
  var relinked = await relinkTracks(config, [projectDir]);
  String? blueprint = blueprintPath == null ? null : await findMedia(blueprintPath, [projectDir]);
  List<String> missing() => [
        ...relinked.missing,
        if (blueprintPath != null && blueprint == null) blueprintPath,
      ];

  if (missing().isNotEmpty) {
    final folder = await askFolder(missing());
    if (folder != null) {
      relinked = await relinkTracks(relinked.config, [folder]);
      if (blueprintPath != null && blueprint == null) {
        blueprint = await findMedia(blueprintPath, [folder]);
      }
    }
  }
  return ProjectMediaResult(
    config: relinked.config,
    blueprintPath: blueprint ?? blueprintPath,
    missing: missing(),
  );
}

/// 다시 연결한 도면 경로를 저장소에 쓴다. 설계 데이터를 복원(restoreExhibitionDataFromFile)한 **다음**,
/// 화면 상태를 다시 읽기 전에 부른다(키는 BlueprintState와 같다).
Future<void> storeBlueprintPath(String? path) async {
  if (path == null) return;
  final prefs = await SharedPreferences.getInstance();
  await prefs.setString('blueprint_image_path', path);
}
