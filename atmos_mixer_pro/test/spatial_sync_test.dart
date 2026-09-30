import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/spatial_sync_provider.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';

/// 공간 설정(스피커 좌표·방·궤적·청취 지점)이 실제로 엔진으로 나가는지 검증한다.
///
/// 예전에는 스피커 배치 상태와 방 상태가 각자 payload를 만들면서 서로를 읽었다.
/// Riverpod이 그걸 순환 의존으로 판단하면 전송이 **조용히** 실패했다 — 화면은
/// 정상인데 소리만 예전 설정으로 남는다. 그 회귀를 잡는 테스트다.
class _FakeEngineState extends EngineStateNotifier {
  @override
  EngineState build() => EngineState(outputChannelCount: 8);
}

void main() {
  ProviderContainer makeContainer() => ProviderContainer(
        // 엔진(FFI)은 테스트에서 못 쓰므로 채널 수만 주는 대역으로 바꾼다.
        overrides: [engineStateProvider.overrideWith(_FakeEngineState.new)],
      );

  RoomZone room() => RoomZone(
        id: 'room_1',
        x: 0,
        y: 0,
        width: 100,
        height: 100,
        color: 0xFF000000,
        physicalWidth: 10,
        physicalHeight: 10,
      );

  test('방과 스피커를 만들면 공간 설정이 엔진으로 나간다', () async {
    SharedPreferences.setMockInitialValues({});
    final container = makeContainer();
    container.read(spatialSyncProvider); // 지연 생성이라 읽어서 활성화해야 한다
    final sync = container.read(spatialSyncProvider.notifier);
    expect(sync.sentCount, 0);

    container.read(roomZoneProvider.notifier).addRoomZone(room());
    container
        .read(speakerLayoutProvider.notifier)
        .addSpeaker(SpeakerNode(id: 'spk_1', roomId: 'room_1', x: 2.0, y: 2.0, channel: 0));
    await Future.delayed(const Duration(milliseconds: 80));

    expect(sync.sentCount, greaterThan(0),
        reason: '상태가 바뀌었는데 공간 설정이 엔진으로 나가지 않았다');
    // 어느 상태도 오류 상태로 빠지지 않아야 한다(순환 의존이면 읽는 순간 터진다).
    expect(container.read(speakerLayoutProvider).length, 1);
    expect(container.read(roomZoneProvider).length, 1);
    container.dispose();
  });

  test('보고 있는 방을 바꾸면 청취 지점을 다시 보낸다', () async {
    SharedPreferences.setMockInitialValues({});
    final container = makeContainer();
    container.read(spatialSyncProvider);
    final sync = container.read(spatialSyncProvider.notifier);
    container.read(roomZoneProvider.notifier).addRoomZone(room());
    await Future.delayed(const Duration(milliseconds: 80));
    final before = sync.sentCount;

    container.read(activeRoomIdProvider.notifier).set('room_1');
    await Future.delayed(const Duration(milliseconds: 80));

    expect(sync.sentCount, greaterThan(before),
        reason: '보고 있는 방이 바뀌면 헤드폰 기준점이 달라지므로 다시 보내야 한다');
    container.dispose();
  });

  test('드래그처럼 연속으로 바뀌어도 한 프레임에 한 번만 보낸다', () async {
    SharedPreferences.setMockInitialValues({});
    final container = makeContainer();
    container.read(spatialSyncProvider);
    final sync = container.read(spatialSyncProvider.notifier);
    final layout = container.read(speakerLayoutProvider.notifier);
    layout.addSpeaker(SpeakerNode(id: 'spk_1', x: 1.0, y: 1.0, channel: 0));
    await Future.delayed(const Duration(milliseconds: 80));
    final before = sync.sentCount;

    for (var i = 0; i < 20; i++) {
      layout.updateSpeaker(
        SpeakerNode(id: 'spk_1', x: 1.0 + i * 0.1, y: 1.0, channel: 0),
        dragging: true,
      );
    }
    await Future.delayed(const Duration(milliseconds: 80));

    final sent = sync.sentCount - before;
    expect(sent, greaterThan(0), reason: '드래그 중에도 소리가 따라와야 한다');
    expect(sent, lessThan(5), reason: '20번 변경마다 보내면 커맨드 채널이 넘친다');
    container.dispose();
  });
}
