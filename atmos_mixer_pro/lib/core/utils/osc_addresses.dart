import 'package:atmos_mixer_pro/src/rust/common/config.dart';

/// OSC 주소 정리(HANDOFF 남은 일 5). 앱은 받은 주소를 글자 그대로 맞추고(osc/router.rs), 같은 주소가 여러 곳에 있으면
/// 마지막에 등록된 하나만 반응한다. 예전에는 새 방이 모두 `/room/clear`, 새 트랙이 모두 `/play`·`/stop`으로 만들어져
/// 사람이 하나하나 바꿔야 했다. 방 볼륨 OSC는 쓰지 않는다(메인 화면에서 조절, 사용자 결정 2026-10-10).

/// 설정이 듣는 OSC 주소들(빈칸 제외, 같은 주소는 여러 번).
List<String> oscAddressesInUse(AppConfig config) => [
      config.themeStartOscAddress,
      config.systemResetOscAddress,
      for (final room in config.rooms) ...[
        room.clearOscAddress,
        for (final track in room.tracks) ...[track.playOscAddress, track.stopOscAddress],
      ],
    ].where((address) => address.isNotEmpty).toList();

/// 두 곳 이상에서 쓰는 주소. 이 주소로는 마지막에 등록된 하나만 반응한다.
Set<String> duplicateOscAddresses(AppConfig config) {
  final seen = <String>{};
  final duplicates = <String>{};
  for (final address in oscAddressesInUse(config)) {
    if (!seen.add(address)) duplicates.add(address);
  }
  return duplicates;
}

/// 방의 번호. 비우기 주소가 `/room{n}/clear`이면 n, 아니면 순서(1부터).
int oscRoomNumber(RoomConfig room, int index) {
  final match = RegExp(r'^/room(\d+)/clear$').firstMatch(room.clearOscAddress);
  return match == null ? index + 1 : int.parse(match.group(1)!);
}

/// 새 방의 비우기 주소 `/room{n}/clear`. 아직 쓰지 않은 가장 작은 n(방 개수 + 1부터).
String newRoomClearAddress(AppConfig config) {
  final used = oscAddressesInUse(config).toSet();
  var n = config.rooms.length + 1;
  while (used.contains('/room$n/clear')) {
    n++;
  }
  return '/room$n/clear';
}

/// 새 트랙의 재생·정지 주소 `/room{r}/track{t}/play`·`stop`. t는 둘 다 [used]에 없는 가장 작은 수([firstTrack]부터)다.
/// 고른 주소는 [used]에 더한다 — 여러 파일을 한 번에 넣을 때 서로 겹치지 않게.
({String play, String stop}) newTrackOscAddresses({
  required int roomNumber,
  required int firstTrack,
  required Set<String> used,
}) {
  var t = firstTrack;
  while (used.contains('/room$roomNumber/track$t/play') || used.contains('/room$roomNumber/track$t/stop')) {
    t++;
  }
  final play = '/room$roomNumber/track$t/play';
  final stop = '/room$roomNumber/track$t/stop';
  used
    ..add(play)
    ..add(stop);
  return (play: play, stop: stop);
}
