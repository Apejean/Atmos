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

/// 프로젝트를 열 때 쓸 오디오 장치를 고른 결과.
class ProjectDeviceChoice {
  const ProjectDeviceChoice({required this.config, required this.keptCurrent, required this.projectDevice});

  /// 장치를 정한 엔진 설정.
  final AppConfig config;

  /// 프로젝트의 장치가 이 PC에 없어 지금 쓰던 장치와 버퍼를 그대로 썼는가.
  final bool keptCurrent;

  /// 프로젝트에 저장된 장치 이름(안내용). 기본 장치면 null.
  final String? projectDevice;
}

/// 프로젝트에 저장된 오디오 장치가 이 PC의 장치 목록에 없으면 지금 쓰던 장치와 버퍼를 그대로 쓴다.
///
/// 맥에서 저장한 프로젝트(`[CoreAudio] Scarlett 6i6 USB`)를 Windows에서 열면, 예전에는 엔진이 그 장치를
/// 30초 동안 찾으며 장치 목록을 0.5초마다 다시 읽었다. 그때마다 ASIO 드라이버를 전부 불러와 Generic Low
/// Latency ASIO 창이 반복해서 떴고, 끝내 "장치를 찾지 못함" 오류로 엔진이 멈춰 소리가 나지 않았다
/// (2026-10-10 Windows P6-E). 장치는 PC마다 다르므로(현장 흐름: 인터페이스 다시 스캔·채널 연결) 그 PC의
/// 장치를 지키고, 다른 장치는 환경설정에서 고른다.
///
/// - 프로젝트가 기본 장치(null)를 쓰면 그대로 둔다(어느 PC에나 있다).
/// - 장치 목록을 모르거나([available] null) 지금 설정이 없으면([current] null) 프로젝트 값을 그대로 둔다.
/// - ASIO 엔진이 도는 동안의 장치 목록에는 쓰고 있는 ASIO 장치만 들어 있다(다른 ASIO 드라이버를 열면 잠금이
///   깨진다). 그래서 다른 ASIO 장치로 저장한 프로젝트도 지금 장치를 지킨다.
ProjectDeviceChoice deviceForImportedProject({
  required AppConfig imported,
  required AppConfig? current,
  required List<String>? available,
}) {
  final wanted = imported.deviceName;
  if (wanted == null || available == null || current == null) {
    return ProjectDeviceChoice(config: imported, keptCurrent: false, projectDevice: wanted);
  }
  bool same(String? a) => a != null && a.trim() == wanted.trim();
  if (same(current.deviceName) || available.any(same)) {
    return ProjectDeviceChoice(config: imported, keptCurrent: false, projectDevice: wanted);
  }
  final c = imported;
  return ProjectDeviceChoice(
    config: AppConfig(
      oscPort: c.oscPort,
      deviceName: current.deviceName,
      bufferSize: current.bufferSize,
      themeStartOscAddress: c.themeStartOscAddress,
      systemResetOscAddress: c.systemResetOscAddress,
      trackingOscAddress: c.trackingOscAddress,
      monoConfigs: c.monoConfigs,
      stereoConfigs: c.stereoConfigs,
      multiConfigs: c.multiConfigs,
      rooms: c.rooms,
      globalTrajectory: c.globalTrajectory,
      roomZones: c.roomZones,
      isExhibitionMode: c.isExhibitionMode,
      masterHeadroomDb: c.masterHeadroomDb,
      peakLimiterEnabled: c.peakLimiterEnabled,
      oscWhitelist: c.oscWhitelist,
      globalReverbMix: c.globalReverbMix,
      globalReverbDecay: c.globalReverbDecay,
    ),
    keptCurrent: true,
    projectDevice: wanted,
  );
}

/// 프로젝트가 쓰는 가장 높은 채널(1부터 센 번호). 트랙 출력(스테레오는 다음 채널까지)과 출력 설정에서 켠
/// 채널(모노 k, 스테레오 k·k+1, 멀티는 시작 채널 k)을 본다. 예전 화면이 남긴 255번 같은 자리표는 세지 않는다.
/// 멀티 채널 파일의 실제 폭은 파일을 열어야 알 수 있어 시작 채널까지만 센다.
int highestProjectChannel(AppConfig config) {
  const placeholder = 255;
  var highest = 0;
  void see(int channel) {
    if (channel < placeholder && channel > highest) highest = channel;
  }

  for (final room in config.rooms) {
    for (final track in room.tracks) {
      see(track.outputChannel + (track.outputStereo ? 2 : 1));
    }
  }
  config.monoConfigs.forEach((k, s) {
    if (s.enabled) see(k);
  });
  config.stereoConfigs.forEach((k, s) {
    if (s.enabled && k < placeholder) see(k + 1);
  });
  config.multiConfigs.forEach((k, s) {
    if (s.enabled) see(k);
  });
  return highest;
}

/// 프로젝트가 지금 장치보다 많은 채널을 쓰면 안내 문장을, 아니면 null을 돌려준다. 장치 채널 수를 모르면(0) null.
String? channelShortfallNotice({required int projectHighest, required int deviceChannels}) {
  if (deviceChannels <= 0 || projectHighest <= deviceChannels) return null;
  return '프로젝트는 CH$projectHighest까지 쓰는데 지금 장치는 $deviceChannels채널이라 '
      'CH${deviceChannels + 1}~CH$projectHighest는 소리가 나지 않습니다.';
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
