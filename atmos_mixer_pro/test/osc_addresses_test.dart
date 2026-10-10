import 'package:atmos_mixer_pro/core/utils/osc_addresses.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';
import 'package:flutter_test/flutter_test.dart';

/// OSC 주소 정리(HANDOFF 남은 일 5, 사용자 요청 2026-10-10): 새 방·트랙의 주소는 겹치지 않게 만들고, 겹치는 주소를 찾는다.
/// 같은 주소가 여러 곳이면 앱은 마지막에 등록된 하나만 반응한다.
void main() {
  TrackConfig track(String id, String play, String stop) => TrackConfig(
        id: id,
        name: id,
        filePath: '/x/$id.wav',
        volume: 1.0,
        isLoop: false,
        isStreaming: false,
        outputChannel: 0,
        outputStereo: true,
        playOscAddress: play,
        stopOscAddress: stop,
      );
  RoomConfig room(String id, String clear, List<TrackConfig> tracks, {String volume = ''}) => RoomConfig(
        id: id,
        name: id,
        colorHex: '#123456',
        volume: 1.0,
        volumeOscAddress: volume,
        clearOscAddress: clear,
        tracks: tracks,
      );
  AppConfig config(List<RoomConfig> rooms, {String theme = '/theme/start', String reset = '/system/reset'}) => AppConfig(
        oscPort: 8000,
        bufferSize: 1024,
        themeStartOscAddress: theme,
        systemResetOscAddress: reset,
        monoConfigs: const {},
        stereoConfigs: const {},
        multiConfigs: const {},
        rooms: rooms,
        roomZones: const [],
        isExhibitionMode: true,
        masterHeadroomDb: 0.0,
        peakLimiterEnabled: true,
        oscWhitelist: const [],
        globalReverbMix: 0.0,
        globalReverbDecay: 1.0,
      );

  test('겹치는 주소를 찾는다(빈칸과 방 볼륨 주소는 보지 않는다)', () {
    final c = config([
      room('r1', '/room/clear', [track('a', '/play', '/stop'), track('b', '/play', '/stop'), track('c', '', '')],
          volume: '/room/volume'),
      room('r2', '/room/clear', [track('d', '/play5', '/stop5')], volume: '/room/volume'),
    ]);
    expect(duplicateOscAddresses(c), {'/room/clear', '/play', '/stop'});
    expect(duplicateOscAddresses(config([room('r1', '/room1/clear', [track('a', '/p1', '/s1')])])), isEmpty);
  });

  test('새 방은 쓰지 않은 /room{n}/clear를 받는다', () {
    expect(newRoomClearAddress(config([])), '/room1/clear');
    expect(newRoomClearAddress(config([room('r1', '/room1/clear', []), room('r2', '/room2/clear', [])])), '/room3/clear');
    // 방 2를 지운 뒤 다시 만들면 남은 방 수(2)+1인 3이 아니라도, 이미 쓰는 번호는 건너뛴다
    expect(newRoomClearAddress(config([room('r1', '/room1/clear', []), room('r3', '/room3/clear', [])])), '/room4/clear');
  });

  test('새 트랙은 방 번호를 따르고 한 번에 여러 개를 넣어도 겹치지 않는다', () {
    final c = config([
      room('r1', '/room1/clear', []),
      room('r3', '/room3/clear', [track('a', '/room3/track1/play', '/room3/track1/stop')]),
    ]);
    expect(oscRoomNumber(c.rooms[1], 1), 3, reason: '비우기 주소의 번호를 쓴다');
    expect(oscRoomNumber(room('x', '/custom/clear', []), 4), 5, reason: '아니면 순서');

    final used = oscAddressesInUse(c).toSet();
    final first = newTrackOscAddresses(roomNumber: 3, firstTrack: 2, used: used);
    final second = newTrackOscAddresses(roomNumber: 3, firstTrack: 2, used: used);
    expect((first.play, first.stop), ('/room3/track2/play', '/room3/track2/stop'));
    expect((second.play, second.stop), ('/room3/track3/play', '/room3/track3/stop'));
    expect(newTrackOscAddresses(roomNumber: 3, firstTrack: 1, used: used).play, '/room3/track4/play',
        reason: '이미 쓰는 번호는 건너뛴다');
  });
}
