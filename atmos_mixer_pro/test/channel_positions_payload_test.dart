import 'package:flutter_test/flutter_test.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/field_geometry.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/position_eq.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart' show EqType;
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';

void main() {
  group('buildChannelPositionsPayload', () {
    test('returns null for channels without a speaker node', () {
      final payload = buildChannelPositionsPayload(const [], 4);
      expect(payload, [null, null, null, null]);
    });

    test('x/y는 이미 미터이므로 변환 없이 그대로 실린다', () {
      // 이 테스트는 예전에 "픽셀을 미터로 변환한다"를 검증했다. 그 전제가
      // 틀렸다. SpeakerNode.x/y는 생성 시점부터 미터다
      // (dynamic_3d_room.dart의 `x: roomWidth * 0.25`), 3D 룸 드래그도
      // 미터로 되돌려준다(studio_engine.html의 `x = newPos.x + width/2`).
      // 픽셀로 오해해 50으로 나누던 탓에 엔진에 가는 좌표가 50배 작아졌고,
      // 절대 거리가 필요한 계산(초기반사음 경로, 거리 감쇠, 시간 정렬)이
      // 전부 무의미해졌다.
      final node = SpeakerNode(id: 'spk_0', x: 10.0, y: 5.0, channel: 0, heightZ: 3.5);

      final entry = buildChannelPositionsPayload([node], 1)[0]!;

      expect(entry['x'], 10.0, reason: '미터 좌표를 다시 변환하면 안 된다');
      expect(entry['y'], 5.0, reason: '미터 좌표를 다시 변환하면 안 된다');
      expect(entry['z'], 3.5, reason: 'heightZ도 이미 미터다');
    });

    test('preserves full height/rotation/tilt/dispersion schema, not zeroed out', () {
      final node = SpeakerNode(
        id: 'spk_0',
        x: 10.0,
        y: 20.0,
        channel: 0,
        heightZ: 3.5,
        rotation: 45.0,
        pitchTilt: 12.5,
        dispersionAngle: 90.0,
      );

      final payload = buildChannelPositionsPayload([node], 1);

      expect(payload.length, 1);
      final entry = payload[0]!;
      expect(entry['x'], 10.0);
      expect(entry['y'], 20.0);
      expect(entry['z'], 3.5, reason: 'z must come from heightZ, not hardcoded 0.0');
      expect(entry['yaw_rotation'], 45.0, reason: 'rotation must not be silently reset to 0');
      expect(entry['pitch_tilt'], 12.5, reason: 'pitchTilt must not be silently reset to 0');
      expect(entry['dispersion_angle'], 90.0, reason: 'dispersionAngle must not be silently reset to 0');
    });

    test('스피커가 속한 방 ID를 방 payload와 같은 값으로 싣는다', () {
      // 방마다 로컬 좌표(원점 0,0)라 엔진에서 RoomZone들이 겹친다. 엔진은
      // 이 room_id로 스피커를 자기 방에 연결한다(좌표로는 구분 불가).
      RoomZone room(String id) => RoomZone(
          id: id, x: 0, y: 0, width: 100, height: 100, color: 0xFFFFFFFF);
      final rooms = [room('room_big'), room('room_small')];
      final roomsPayload = buildRoomZonesPayload(rooms);

      final nodes = [
        SpeakerNode(id: 'a', roomId: 'room_small', x: 1, y: 1, channel: 0),
        SpeakerNode(id: 'b', roomId: 'room_big', x: 1, y: 1, channel: 1),
        SpeakerNode(id: 'c', x: 1, y: 1, channel: 2), // 방이 없는 예전 스피커
      ];
      final payload = buildChannelPositionsPayload(nodes, 3);

      expect(payload[0]!['room_id'], roomsPayload[1]['room_id']);
      expect(payload[1]!['room_id'], roomsPayload[0]['room_id']);
      expect(payload[0]!['room_id'], isNot(payload[1]!['room_id']));
      expect(payload[2]!['room_id'], isNull, reason: '방이 없으면 엔진이 좌표로 찾도록 비워 둔다');
    });

    test('같은 채널을 여러 방에 쓰면 지금 보고 있는 방의 스피커를 보낸다', () {
      // 출력이 적은 장비로 설계할 때 같은 채널을 여러 방에 쓰기도 한다(테마 1과 2번 방의 CH2).
      final nodes = [
        SpeakerNode(id: 'a', roomId: 'A', x: 1, y: 1, channel: 1),
        SpeakerNode(id: 'b', roomId: 'B', x: 4, y: 4, channel: 1),
      ];
      expect(buildChannelPositionsPayload(nodes, 2, activeRoomId: 'A')[1]!['x'], 1);
      expect(buildChannelPositionsPayload(nodes, 2, activeRoomId: 'B')[1]!['x'], 4);
      expect(buildChannelPositionsPayload(nodes, 2)[1]!['x'], 1,
          reason: '보고 있는 방에 그 채널이 없으면 목록의 첫 스피커');
    });
    test('헤드폰 현장 물리 밴드를 스피커마다 sim_bands로 싣는다(슬롯 고정)', () {
      // 벽에서 0.3m: 경계면 저음 증가가 생긴다.
      final node = SpeakerNode(id: 'a', roomId: 'A', x: 0.3, y: 5.0, channel: 0);
      const g = FieldGeometry(
        roomWidth: 10, roomDepth: 10, ceilingHeight: 3,
        listenerX: 5, listenerY: 5, listenerZ: 1.2, absorption: 0.3,
      );
      final payload = buildChannelPositionsPayload(
        [node], 1,
        simBands: (n) => fieldPhysicsPayloadFor(n, g, 343.0),
      );
      final bands = payload[0]!['sim_bands'] as List;
      expect(bands.length, kFieldPhysicsBandCount);
      expect(bands[kPhysicsSlotBoundary]['on'], isTrue);
      expect(bands[kPhysicsSlotBoundary]['type'], EqType.lowShelf.index);
      expect(bands[kPhysicsSlotBoundary]['gain'], greaterThan(0.0));
      expect(bands[kPhysicsSlotDirectivity]['type'], EqType.highShelf.index);

      // 물리 밴드를 넘기지 않으면 싣지 않는다(예전 payload와 같다).
      expect(buildChannelPositionsPayload([node], 1)[0]!.containsKey('sim_bands'), isFalse);
    });
  });
}

