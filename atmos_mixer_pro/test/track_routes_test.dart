import 'package:atmos_mixer_pro/core/utils/track_routes.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';
import 'package:flutter_test/flutter_test.dart';

/// 매트릭스 라우팅 표의 순수 함수(설계 `docs/superpowers/specs/2026-11-14-matrix-routing-design.md` 7.1·7.3, 개정 3).
void main() {
  TrackRoute r(int f, int o, [double g = 1.0]) => TrackRoute(fileChannel: f, outputChannel: o, gain: g);

  group('seedRoutesFromLegacy — 매핑과 같은 소리가 나는 표(7.3, 개정 3)', () {
    test('모노 파일은 출력 하나(스테레오 출력 표시가 있어도, 1.3.18부터)', () {
      expect(seedRoutesFromLegacy(fileChannels: 1, outputChannel: 2, outputStereo: false), [r(0, 2)]);
      expect(seedRoutesFromLegacy(fileChannels: 1, outputChannel: 2, outputStereo: true), [r(0, 2)]);
    });
    test('2채널 이상 + 스테레오/멀티 출력은 N:N', () {
      expect(seedRoutesFromLegacy(fileChannels: 2, outputChannel: 2, outputStereo: true), [r(0, 2), r(1, 3)]);
      expect(seedRoutesFromLegacy(fileChannels: 4, outputChannel: 0, outputStereo: true),
          [r(0, 0), r(1, 1), r(2, 2), r(3, 3)]);
    });
    test('2채널 이상 + 모노 출력은 한 출력에 1/N씩(평균과 같은 소리)', () {
      expect(seedRoutesFromLegacy(fileChannels: 2, outputChannel: 3, outputStereo: false), [r(0, 3, 0.5), r(1, 3, 0.5)]);
    });
  });

  test('normalizeRoutes: 0 이하 제거, 1 초과는 1, 겹치면 뒤의 것, (출력, 파일) 오름차순', () {
    expect(
      normalizeRoutes([r(0, 5), r(1, 2, 0.0), r(0, 2, 1.5), r(0, 5, 0.3), r(2, 1, -1)]),
      [r(0, 2, 1.0), r(0, 5, 0.3)],
    );
  });

  test('toggleRoute: 없으면 100%로 더하고 있으면 빼되, 마지막 칸은 빼지 않는다(D12)', () {
    var routes = [r(0, 0)];
    routes = toggleRoute(routes, 0, 2);
    expect(routes, [r(0, 0), r(0, 2)]);
    routes = toggleRoute(routes, 0, 0);
    expect(routes, [r(0, 2)]);
    expect(toggleRoute(routes, 0, 2), [r(0, 2)], reason: '마지막 칸');
  });

  test('setRouteGain은 1~100%로 자르고 그 칸만 바꾼다', () {
    final routes = [r(0, 0), r(0, 1)];
    expect(setRouteGain(routes, 0, 1, 0.7), [r(0, 0), r(0, 1, 0.7)]);
    expect(setRouteGain(routes, 0, 1, 0.0), [r(0, 0), r(0, 1, 0.01)]);
    expect(setRouteGain(routes, 0, 1, 3.0), [r(0, 0), r(0, 1, 1.0)]);
    expect(setRouteGain(routes, 1, 1, 0.5), routes, reason: '없는 칸은 그대로');
  });

  test('outputGainSums는 출력마다 음량을 더한다(합 경고용)', () {
    expect(outputGainSums([r(0, 1, 1.0), r(1, 1, 0.5), r(0, 3, 0.2)]), {1: 1.5, 3: 0.2});
  });

  test('routesSummary·highestRouteChannel', () {
    expect(routesSummary([r(0, 4), r(0, 0), r(0, 2)]), '표 · Ch-1, 3, 5');
    expect(routesSummary([for (var o = 0; o < 8; o++) r(0, o)]), '표 · Ch-1, 2, 3, 4, 5, 6 … 외 2');
    expect(routesSummary([r(0, 1), r(1, 1)]), '표 · Ch-2', reason: '같은 출력은 한 번');
    expect(highestRouteChannel([r(0, 4), r(0, 9)]), 10);
    expect(highestRouteChannel(const []), 0);
  });

  test('routeProblems는 파일에 없는 채널·장치에 없는 출력·닫힌 출력을 가른다', () {
    final p = routeProblems(
      [r(0, 0), r(3, 1), r(0, 2), r(0, 9)],
      fileChannels: 2,
      enabled: [true, true, false, true],
      deviceChannels: 4,
    );
    expect(p.missingFileChannel, [r(3, 1)]);
    expect(p.closedOutput, [r(0, 2)]);
    expect(p.missingOutput, [r(0, 9)]);
    expect(p.isEmpty, isFalse);
    expect(routeProblems([r(0, 0)], fileChannels: null, enabled: [true], deviceChannels: 1).isEmpty, isTrue);
  });

  test('seedMissingRoutes: 표가 없고 채널 수를 아는 트랙만 채우고, 매핑 값과 있던 표는 그대로', () {
    TrackConfig t(String id, int out, bool stereo, List<TrackRoute> routes) => TrackConfig(
          id: id,
          name: id,
          filePath: '$id.wav',
          volume: 1,
          isLoop: true,
          isStreaming: true,
          outputChannel: out,
          outputStereo: stereo,
          playOscAddress: '',
          stopOscAddress: '',
          routes: routes,
        );
    final rooms = [
      RoomConfig(id: 'r', name: 'r', colorHex: '#000000', volume: 1, clearOscAddress: '', tracks: [
        t('mono', 4, false, const []),
        t('stereo', 0, true, const []),
        t('has', 0, true, [r(0, 7)]),
        t('unknown', 2, false, const []),
      ]),
    ];
    final seeded = seedMissingRoutes(rooms, {'mono': 1, 'stereo': 2, 'has': 2});
    final tracks = seeded.rooms.single.tracks;
    expect(tracks[0].routes, [r(0, 4)]);
    expect(tracks[1].routes, [r(0, 0), r(1, 1)]);
    expect(tracks[2].routes, [r(0, 7)], reason: '이미 있던 표');
    expect(tracks[3].routes, isEmpty, reason: '채널 수를 몰라 비워 둔다');
    for (var i = 0; i < 4; i++) {
      expect((tracks[i].outputChannel, tracks[i].outputStereo),
          (rooms.single.tracks[i].outputChannel, rooms.single.tracks[i].outputStereo),
          reason: '매핑 값은 그대로(D14)');
    }
    expect((seeded.filled, seeded.unread), (2, 1));
  });
}
