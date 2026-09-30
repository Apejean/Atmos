import 'package:flutter_test/flutter_test.dart';

import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';

/// 헤드폰 미리듣기(바이노럴)는 "지금 보고 있는 방 중심에 서 있는" 것으로 계산해야 한다.
/// 예전에는 항상 첫 번째 방 중심을 썼기 때문에, 2번 방 이후를 설계할 때 방향이 틀렸다.
void main() {
  RoomZone room(String id, double w, double h, double ear) => RoomZone(
        id: id,
        x: 0,
        y: 0,
        width: 100,
        height: 100,
        color: 0xFF000000,
        physicalWidth: w,
        physicalHeight: h,
        earLevel: ear,
      );

  final rooms = [room('room_a', 20.0, 15.0, 1.6), room('room_b', 6.0, 4.0, 1.2)];

  test('보고 있는 방의 중심과 귀 높이를 청취 지점으로 보낸다', () {
    final payload = buildListenerPayload(
      rooms: rooms,
      activeRoomId: 'room_b',
      fallbackWidth: 40.0,
      fallbackHeight: 40.0,
    );

    expect(payload['listener_position']['x'], 3.0);
    expect(payload['listener_position']['y'], 2.0);
    expect(payload['listener_position']['z'], 1.2);
    expect(payload['active_room_id'], engineRoomId('room_b'));
  });

  test('보고 있는 방이 정해지지 않았으면 첫 번째 방을 쓴다', () {
    final payload = buildListenerPayload(
      rooms: rooms,
      activeRoomId: null,
      fallbackWidth: 40.0,
      fallbackHeight: 40.0,
    );

    expect(payload['listener_position']['x'], 10.0);
    expect(payload['listener_position']['z'], 1.6);
    expect(payload['active_room_id'], engineRoomId('room_a'));
  });

  test('방이 하나도 없으면 청사진 캔버스 중심을 쓴다', () {
    final payload = buildListenerPayload(
      rooms: const [],
      activeRoomId: null,
      fallbackWidth: 30.0,
      fallbackHeight: 20.0,
    );

    expect(payload['listener_position']['x'], 15.0);
    expect(payload['listener_position']['y'], 10.0);
    expect(payload['active_room_id'], isNull);
  });
}
