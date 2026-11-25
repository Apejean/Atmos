import 'package:atmos_mixer_pro/src/rust/common/config.dart';

/// 매트릭스 라우팅 연결 표의 순수 함수(설계 `docs/superpowers/specs/2026-11-14-matrix-routing-design.md` 7.1, 개정 3).
/// 표 창·트랙 카드·환경설정이 같은 함수를 쓴다. 엔진이 받아들이는 규칙(audio::route::resolve_routes)과 같은 방향으로
/// 정리한다(엔진이 최종 검증을 다시 한다).

/// 0 이하 음량은 빼고 1을 넘으면 1로, 같은 (파일, 출력)은 뒤의 것만 남기고 (출력, 파일) 오름차순으로 둔다.
List<TrackRoute> normalizeRoutes(List<TrackRoute> routes) {
  final byCell = <(int, int), TrackRoute>{};
  for (final r in routes) {
    if (!r.gain.isFinite || r.gain <= 0) continue;
    byCell[(r.fileChannel, r.outputChannel)] =
        TrackRoute(fileChannel: r.fileChannel, outputChannel: r.outputChannel, gain: r.gain > 1 ? 1.0 : r.gain);
  }
  final out = byCell.values.toList()
    ..sort((a, b) {
      final o = a.outputChannel.compareTo(b.outputChannel);
      return o != 0 ? o : a.fileChannel.compareTo(b.fileChannel);
    });
  return out;
}

/// 트랙별 출력 매핑과 같은 소리가 나는 표(설계 7.3, 개정 3 ④): 모노 파일은 출력 하나(1.3.18부터 옆 채널로 올리지
/// 않는다), 2채널 이상 + 스테레오/멀티 출력은 파일 채널 c → 출력 + c, 2채널 이상 + 모노 출력은 한 출력에 1/N씩.
List<TrackRoute> seedRoutesFromLegacy({
  required int fileChannels,
  required int outputChannel,
  required bool outputStereo,
}) {
  final n = fileChannels < 1 ? 1 : fileChannels;
  if (n == 1) {
    return [TrackRoute(fileChannel: 0, outputChannel: outputChannel, gain: 1.0)];
  }
  if (outputStereo) {
    return [for (var c = 0; c < n; c++) TrackRoute(fileChannel: c, outputChannel: outputChannel + c, gain: 1.0)];
  }
  return [for (var c = 0; c < n; c++) TrackRoute(fileChannel: c, outputChannel: outputChannel, gain: 1.0 / n)];
}

/// 칸이 없으면 100%로 더하고, 있으면 뺀다. 빼서 0개가 되면 원래 목록을 그대로 돌려준다(마지막 칸은 끊을 수 없다, D12).
List<TrackRoute> toggleRoute(List<TrackRoute> routes, int fileChannel, int outputChannel) {
  final exists = routes.any((r) => r.fileChannel == fileChannel && r.outputChannel == outputChannel);
  if (!exists) {
    return normalizeRoutes([...routes, TrackRoute(fileChannel: fileChannel, outputChannel: outputChannel, gain: 1.0)]);
  }
  final rest = [
    for (final r in routes)
      if (!(r.fileChannel == fileChannel && r.outputChannel == outputChannel)) r,
  ];
  return rest.isEmpty ? routes : normalizeRoutes(rest);
}

/// 칸 하나의 음량을 1~100%로 잘라 바꾼다. 없는 칸이면 그대로.
List<TrackRoute> setRouteGain(List<TrackRoute> routes, int fileChannel, int outputChannel, double gain) {
  final g = gain.isFinite ? gain.clamp(0.01, 1.0).toDouble() : 1.0;
  return [
    for (final r in routes)
      if (r.fileChannel == fileChannel && r.outputChannel == outputChannel)
        TrackRoute(fileChannel: r.fileChannel, outputChannel: r.outputChannel, gain: g)
      else
        r,
  ];
}

/// 출력마다 칸 음량의 합(합 100% 초과 경고용).
Map<int, double> outputGainSums(List<TrackRoute> routes) {
  final sums = <int, double>{};
  for (final r in routes) {
    sums[r.outputChannel] = (sums[r.outputChannel] ?? 0) + r.gain;
  }
  return sums;
}

/// 트랙 카드·환경설정에 보이는 요약: `표 · Ch-1, 3, 5`(여섯 개를 넘으면 `… 외 n`).
String routesSummary(List<TrackRoute> routes) {
  final outs = routes.map((r) => r.outputChannel + 1).toSet().toList()..sort();
  if (outs.isEmpty) return '표 · 연결 없음';
  const shown = 6;
  final head = outs.take(shown).join(', ');
  final more = outs.length > shown ? ' … 외 ${outs.length - shown}' : '';
  return '표 · Ch-$head$more';
}

/// 표가 가리키는 가장 높은 출력(1부터 센 번호). 없으면 0.
int highestRouteChannel(List<TrackRoute> routes) =>
    routes.fold(0, (m, r) => r.outputChannel + 1 > m ? r.outputChannel + 1 : m);

/// 소리가 나지 않는 칸의 분류(표 창의 주황 행·열, 트랙 카드 요약 색).
class RouteProblems {
  const RouteProblems({required this.missingFileChannel, required this.missingOutput, required this.closedOutput});

  /// 파일에 없는 채널을 가리키는 칸.
  final List<TrackRoute> missingFileChannel;

  /// 지금 장치에 없는 출력을 가리키는 칸.
  final List<TrackRoute> missingOutput;

  /// 출력 설정(Output Config)에서 닫힌 출력을 가리키는 칸.
  final List<TrackRoute> closedOutput;

  bool get isEmpty => missingFileChannel.isEmpty && missingOutput.isEmpty && closedOutput.isEmpty;
}

/// 표의 문제 칸을 가른다. `enabled`는 엔진과 같은 규칙의 열린 출력(`computeEnabledChannels`), `fileChannels`를
/// 모르면 파일 채널 검사는 하지 않는다.
RouteProblems routeProblems(
  List<TrackRoute> routes, {
  required int? fileChannels,
  required List<bool> enabled,
  required int deviceChannels,
}) {
  final missingFile = <TrackRoute>[];
  final missingOut = <TrackRoute>[];
  final closed = <TrackRoute>[];
  for (final r in routes) {
    if (fileChannels != null && r.fileChannel >= fileChannels) {
      missingFile.add(r);
    } else if (r.outputChannel >= deviceChannels) {
      missingOut.add(r);
    } else if (r.outputChannel >= enabled.length || !enabled[r.outputChannel]) {
      closed.add(r);
    }
  }
  return RouteProblems(missingFileChannel: missingFile, missingOutput: missingOut, closedOutput: closed);
}

/// 매트릭스로 바꿀 때 표가 없는 트랙을 지금 매핑과 같은 연결로 채운다(설계 D15·7.0). 채널 수를 모르는 트랙은 비워 두고
/// (엔진은 매핑대로 낸다), 이미 표가 있는 트랙과 모든 매핑 값은 그대로 둔다. 채운 수와 못 채운 수를 함께 돌려준다.
({List<RoomConfig> rooms, int filled, int unread}) seedMissingRoutes(
  List<RoomConfig> rooms,
  Map<String, int> fileChannels,
) {
  var filled = 0;
  var unread = 0;
  final out = [
    for (final r in rooms)
      RoomConfig(
        id: r.id,
        name: r.name,
        colorHex: r.colorHex,
        volume: r.volume,
        clearOscAddress: r.clearOscAddress,
        tracks: [
          for (final t in r.tracks)
            if (t.routes.isNotEmpty)
              t
            else if (fileChannels[t.id] == null)
              t
            else
              TrackConfig(
                id: t.id,
                name: t.name,
                filePath: t.filePath,
                volume: t.volume,
                isLoop: t.isLoop,
                isStreaming: t.isStreaming,
                outputChannel: t.outputChannel,
                outputStereo: t.outputStereo,
                playOscAddress: t.playOscAddress,
                stopOscAddress: t.stopOscAddress,
                routes: seedRoutesFromLegacy(
                  fileChannels: fileChannels[t.id]!,
                  outputChannel: t.outputChannel,
                  outputStereo: t.outputStereo,
                ),
              ),
        ],
      ),
  ];
  for (final r in rooms) {
    for (final t in r.tracks) {
      if (t.routes.isNotEmpty) continue;
      if (fileChannels[t.id] == null) {
        unread++;
      } else {
        filled++;
      }
    }
  }
  return (rooms: out, filled: filled, unread: unread);
}
