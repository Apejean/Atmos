import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/settings/widgets/tuning_modal.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/acoustic_sync_provider.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';

import 'dart:math' as math;

import 'package:atmos_mixer_pro/features/exhibition/state/environment_state_provider.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// 스피커 위치가 바뀌면 채널별 FX(딜레이/게인 등)가 자동으로 따라오는지 검증한다.
///
/// 이 테스트는 예전에 값을 print만 하고 아무것도 단언하지 않아 항상 통과했다.
/// 게다가 방을 `updateRoomZone`(기존 방 갱신용)으로 넣어서 실제로는 방이
/// 추가되지 않았고, 동기화 코드가 `rooms.isEmpty`에서 그냥 빠져나가 한 번도
/// 실행되지 않았다. 그래서 출력이 전부 null이었다.
///
/// 정작 제품에서는 `acousticSyncProvider`를 앱 어디에서도 읽지 않아(테스트
/// 에서만 읽었다) Riverpod 지연 생성 때문에 기능 전체가 죽어 있었다.
/// 지금은 메인 화면에서 watch한다.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  /// 채널 0짜리 스피커 하나를 10m x 10m 방에 두고 동기화를 켠다.
  Future<ProviderContainer> setUpContainer() async {
    SharedPreferences.setMockInitialValues({});
    final container = ProviderContainer();

    final room = RoomZone(
      id: 'room1',
      x: 0,
      y: 0,
      width: 10,
      height: 10,
      color: 0xFF0000,
      physicalWidth: 10,
      physicalHeight: 10,
    );
    container.read(roomZoneProvider.notifier).addRoomZone(room);

    final speaker = SpeakerNode(id: 'spk1', roomId: 'room1', x: 2.0, y: 2.0, channel: 0);
    container.read(speakerLayoutProvider.notifier).addSpeaker(speaker);

    // provider를 활성화해야 ref.listen이 등록된다(지연 생성).
    container.read(acousticSyncProvider);
    await Future.delayed(const Duration(milliseconds: 60));
    return container;
  }

  test('스피커를 옮기면 해당 채널의 FX 값이 실제로 재계산된다', () async {
    final container = await setUpContainer();

    // 채널 0 -> 튜닝 맵 키는 1 (UI가 1-based로 표시하는 관례).
    final before = container.read(tuningStateProvider)[1];
    expect(
      before,
      isNotNull,
      reason: '동기화가 돌았다면 채널 1의 튜닝이 만들어져 있어야 한다. '
          'null이면 acousticSyncProvider가 실행되지 않은 것이다.',
    );
    final beforeDelay = before!.delay;

    // 스피커를 리스너에서 훨씬 먼 쪽으로 옮긴다 -> 시간 정렬 딜레이가 달라져야 한다.
    container.read(speakerLayoutProvider.notifier).updateSpeaker(
          SpeakerNode(id: 'spk1', roomId: 'room1', x: 9.0, y: 9.0, channel: 0),
        );
    await Future.delayed(const Duration(milliseconds: 60));

    final after = container.read(tuningStateProvider)[1];
    expect(after, isNotNull);
    expect(
      after!.delay,
      isNot(equals(beforeDelay)),
      reason: '스피커를 옮겼는데 딜레이가 그대로다 — 위치 변경이 FX에 반영되지 않았다. '
          'before=$beforeDelay after=${after.delay}',
    );

    container.dispose();
  });

  test('방이 없으면 계산을 건너뛰고 크래시하지 않는다', () async {
    SharedPreferences.setMockInitialValues({});
    final container = ProviderContainer();

    container.read(speakerLayoutProvider.notifier).addSpeaker(
          SpeakerNode(id: 'spk1', x: 2.0, y: 2.0, channel: 0),
        );
    container.read(acousticSyncProvider);
    await Future.delayed(const Duration(milliseconds: 60));

    // 방 정보가 없으면 음향 계산의 근거가 없으므로 아무것도 쓰지 않는다.
    expect(container.read(tuningStateProvider)[1], isNull);

    container.dispose();
  });

  test('딜레이와 게인이 물리 계산값과 일치한다', () async {
    final container = await setUpContainer();

    final env = container.read(environmentStateProvider);
    final tuning = container.read(tuningStateProvider)[1];
    expect(tuning, isNotNull);

    // 10m x 10m 방의 리스너는 정중앙 (5, 5, earLevel).
    // 스피커는 (2, 2, heightZ 기본 3.5).
    const room = 10.0;
    const earLevel = 1.2; // RoomZone 기본값
    final dx = 2.0 - room / 2.0;
    final dy = 2.0 - room / 2.0;
    final dz = 3.5 - earLevel;
    final distance = math.sqrt(dx * dx + dy * dy + dz * dz);

    final expectedDelay = (distance / env.speedOfSound) * 1000.0;
    final expectedGain = 20.0 * math.log(1.0 / distance) / math.ln10;

    expect(
      tuning!.delay,
      closeTo(expectedDelay, 0.01),
      reason: '딜레이가 비행시간(거리/음속) 계산값과 다르다',
    );
    expect(
      tuning.gainDb,
      closeTo(expectedGain, 0.5),
      reason: '게인이 역제곱 거리 감쇠 계산값과 다르다',
    );

    container.dispose();
  });
}
