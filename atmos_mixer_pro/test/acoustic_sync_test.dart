import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/settings/widgets/tuning_modal.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/acoustic_sync_provider.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/position_eq.dart';

import 'dart:convert';
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

  /// 자동 동기화는 16ms 스로틀 뒤에 비동기로 돈다. 고정 시간을 기다리면 CPU가 바쁠 때
  /// (전체 테스트 동시 실행 등) 간헐적으로 실패하므로, 조건이 만족될 때까지 기다린다.
  Future<void> waitUntil(bool Function() done, {String what = '동기화'}) async {
    final deadline = DateTime.now().add(const Duration(seconds: 3));
    while (!done()) {
      if (DateTime.now().isAfter(deadline)) {
        fail('$what이(가) 3초 안에 끝나지 않았다');
      }
      await Future.delayed(const Duration(milliseconds: 10));
    }
    // 같은 틱에 이어지는 후속 갱신까지 흘려보낸다.
    await Future.delayed(const Duration(milliseconds: 20));
  }

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
    // 첫 자동 계산이 끝나 채널 튜닝이 만들어질 때까지 기다린다.
    await waitUntil(() => container.read(tuningStateProvider)[1] != null,
        what: '첫 자동 계산');
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
    // 정렬은 스피커가 둘 이상일 때만 의미가 있다(한 대뿐이면 그 스피커가
    // 곧 기준이라 딜레이는 항상 0이 맞는 값이다). 기준이 될 먼 스피커를
    // 한 대 더 둔다.
    container.read(speakerLayoutProvider.notifier).addSpeaker(
          SpeakerNode(id: 'spk2', roomId: 'room1', x: 9.8, y: 9.8, channel: 1),
        );
    await Future.delayed(const Duration(milliseconds: 250));

    final baseline = container.read(tuningStateProvider)[1]!;

    // 채널 1 스피커를 리스너 쪽으로 더 붙인다 -> 정렬 딜레이는 늘고,
    // 거리 보정 게인은 더 깎이고, 배치 기반 EQ도 따라 움직여야 한다.
    container.read(speakerLayoutProvider.notifier).updateSpeaker(
          SpeakerNode(id: 'spk1', roomId: 'room1', x: 4.8, y: 4.8, channel: 0),
        );
    await Future.delayed(const Duration(milliseconds: 250));

    final after = container.read(tuningStateProvider)[1];
    expect(after, isNotNull);
    expect(
      after!.delay,
      greaterThan(baseline.delay),
      reason: '리스너에 더 가까워졌으면 정렬 딜레이가 늘어야 한다. '
          'before=${baseline.delay} after=${after.delay}',
    );
    expect(
      after.gainDb,
      lessThan(baseline.gainDb),
      reason: '리스너에 더 가까워졌으면 거리 보정 게인이 더 깎여야 한다.',
    );
    expect(
      after.gains.sublist(0, kAutoEqSlotCount),
      isNot(equals(baseline.gains.sublist(0, kAutoEqSlotCount))),
      reason: '스피커를 옮겼는데 자동 EQ 게인이 하나도 안 바뀌었다 — '
          '위치가 EQ에 반영되지 않는 회귀다.',
    );

    container.dispose();
  });

  test('LFE로 지정해도 채널 게인에 자동 +10dB가 붙지 않는다', () async {
    // 예전에는 LFE로 지정한 채널 게인에 +10dB를 자동으로 더했다. 그 채널이 풀레인지
    // 프로그램을 내고 있으면 중·고역까지 10dB 커져 날카롭게 들렸다. 이제 서브 레벨은
    // 출력단·하드웨어에서 맞추고, 필요하면 베이스 매니지먼트의 LFE +10dB 토글
    // (120Hz 로우패스 이후에만 적용)을 쓴다.
    final container = await setUpContainer();
    final before = container.read(tuningStateProvider)[1]!.gainDb;

    container.read(speakerLayoutProvider.notifier).setSubwoofer('spk1', true);
    await waitUntil(() => true, what: 'LFE 지정 반영');
    await Future.delayed(const Duration(milliseconds: 250));

    final after = container.read(tuningStateProvider)[1]!.gainDb;
    expect(after, closeTo(before, 0.01),
        reason: 'LFE 지정만으로 채널 게인이 바뀌었다: $before -> $after');
    container.dispose();
  });

  test('LFE로 지정된 스피커는 서브용 자동 EQ와 극성을 받는다', () async {
    final container = await setUpContainer();
    final layout = container.read(speakerLayoutProvider.notifier);
    // 청취자(5,5) 뒤쪽 벽 가까이, 저역 한계 120Hz: 일반 스피커라면 극성이 뒤집히고
    // 120Hz 로우컷이 걸린다.
    layout.updateSpeaker(
      SpeakerNode(
          id: 'spk1', roomId: 'room1', x: 5.0, y: 9.8, channel: 0, lowCutHz: 120.0),
    );
    await waitUntil(() => container.read(tuningStateProvider)[1]!.phaseInvert,
        what: '뒤쪽 스피커 극성 반전');
    expect(container.read(tuningStateProvider)[1]!.freqs[kAutoEqSlotLowCut],
        greaterThan(100.0));

    layout.setSubwoofer('spk1', true);
    await waitUntil(
        () => container.read(tuningStateProvider)[1]!.freqs[kAutoEqSlotLowCut] <= 25.0,
        what: '서브용 자동 EQ');
    final sub = container.read(tuningStateProvider)[1]!;
    expect(sub.phaseInvert, isFalse,
        reason: '서브 극성은 위치로 자동 뒤집지 않는다(크로스오버에서 메인과 맞춘다)');
    container.dispose();
  });

  test('앱을 켤 때 저장된 튜닝을 불러온 뒤에 자동 계산한다', () async {
    // 예전에는 자동 계산이 저장값 불러오기보다 먼저 끝나면, 불러오기가 그 결과를 옛
    // 저장값으로 덮어쓰고 엔진에도 옛 값을 다시 보냈다. 그래서 배치·규칙이 바뀐 뒤 앱을
    // 다시 켜도 스피커를 한 번 움직이기 전까지 옛 튜닝이 그대로 걸렸다(실기: 테마 1 CH2에
    // 2번 방 기준 극성 반전·로우컷이 남음).
    final stale = ChannelTuningState.initial().copyWith(gainDb: 9.9);
    final locked = ChannelTuningState.initial()
        .copyWith(gainDb: -7.5, isTuningLocked: true);
    SharedPreferences.setMockInitialValues({
      'tuning_state': jsonEncode({'1': stale.toJson(), '2': locked.toJson()}),
      'tuning_state_schema_version': 4,
    });
    final container = ProviderContainer();
    // 앱 시작과 같은 순서: 자동 계산이 먼저 떠 있고, 배치가 들어오면서 계산이 돈다.
    // 튜닝 상태는 자동 계산이 처음 읽는다(저장값 불러오기가 아직 끝나지 않은 순간).
    container.read(acousticSyncProvider);
    container.read(roomZoneProvider.notifier).addRoomZone(RoomZone(
          id: 'room1',
          x: 0,
          y: 0,
          width: 10,
          height: 10,
          color: 0xFF0000,
          physicalWidth: 10,
          physicalHeight: 10,
        ));
    final layout = container.read(speakerLayoutProvider.notifier);
    layout.addSpeaker(SpeakerNode(id: 'spk1', roomId: 'room1', x: 2.0, y: 2.0, channel: 0));
    layout.addSpeaker(SpeakerNode(id: 'spk2', roomId: 'room1', x: 8.0, y: 2.0, channel: 1));
    // 여기서 튜닝 상태를 읽으면 불러오기가 먼저 시작되어 앱의 순서와 달라진다.
    await Future.delayed(const Duration(milliseconds: 400));

    // 자동 게인 = 20log10(청취 지점까지 거리 / 5m). 스피커 높이 3.5m, 귀 1.2m.
    final dist = math.sqrt(9 + 9 + math.pow(3.5 - 1.2, 2));
    final autoGain = 20 * math.log(dist / 5.0) / math.ln10;

    expect(container.read(tuningStateProvider)[1]!.gainDb, closeTo(autoGain, 0.05),
        reason: '불러온 옛 저장값(9.9dB)이 자동 계산 결과를 덮어썼다');
    expect(container.read(tuningStateProvider)[2]!.gainDb, -7.5,
        reason: '잠근 채널을 자동 계산이 덮어썼다');
    await Future.delayed(const Duration(milliseconds: 700)); // 저장(500ms 디바운스)
    final saved = jsonDecode(
        (await SharedPreferences.getInstance()).getString('tuning_state')!) as Map;
    expect((saved['1']['gainDb'] as num).toDouble(), closeTo(autoGain, 0.05),
        reason: '자동 계산 결과가 저장되지 않았다(다음 실행에 옛 값이 다시 올라온다)');
    container.dispose();
  });

  test('서브가 있는 방의 뒤쪽 메인은 극성을 자동으로 뒤집지 않는다', () async {
    // 엔진은 메인의 저역을 극성 보정 전에 갈라 서브로 보낸다. 뒤쪽 메인만 뒤집으면
    // 크로스오버 지점에서 서브와 반대 극성이 되어 그 대역이 지워진다.
    final container = await setUpContainer();
    final layout = container.read(speakerLayoutProvider.notifier);
    layout.updateSpeaker(
        SpeakerNode(id: 'spk1', roomId: 'room1', x: 5.0, y: 9.8, channel: 0));
    await waitUntil(() => container.read(tuningStateProvider)[1]!.phaseInvert,
        what: '서브 없는 방의 뒤쪽 스피커 극성 반전');

    layout.addSpeaker(SpeakerNode(id: 'sub', roomId: 'room1', x: 5.0, y: 0.5, channel: 2));
    layout.setSubwoofer('sub', true);
    await waitUntil(() => !container.read(tuningStateProvider)[1]!.phaseInvert,
        what: '서브가 생긴 방의 뒤쪽 메인 극성 복원');

    layout.setSubwoofer('sub', false);
    await waitUntil(() => container.read(tuningStateProvider)[1]!.phaseInvert,
        what: '서브를 없애면 다시 뒤집는다');
    container.dispose();
  });

  test('같은 채널을 여러 방에 쓰면 보고 있는 방의 스피커로 튜닝한다', () async {
    // 예전에는 목록의 마지막 스피커 값이 이겨서(엔진 좌표는 첫 스피커) 한 채널에 서로
    // 다른 방 기준 튜닝이 섞였다(실기: 테마 1의 CH2에 2번 방 기준 로우컷·극성·게인).
    final container = await setUpContainer(); // room1(10x10): spk1(ch0) (2,2)
    container.read(roomZoneProvider.notifier).addRoomZone(RoomZone(
          id: 'room2',
          x: 0,
          y: 0,
          width: 5,
          height: 5,
          color: 0x00FF00,
          physicalWidth: 5,
          physicalHeight: 5,
        ));
    container.read(speakerLayoutProvider.notifier).addSpeaker(
        SpeakerNode(id: 'spkB', roomId: 'room2', x: 0.5, y: 0.5, channel: 0));

    // 자동 게인 = 20log10(청취 지점까지 거리 / min(가로, 세로)/2). 스피커 높이 3.5m, 귀 1.2m.
    double expectedGain(double x, double y, double w, double d) {
      final dist = math.sqrt(math.pow(x - w / 2, 2) + math.pow(y - d / 2, 2) + math.pow(3.5 - 1.2, 2));
      return 20 * math.log(dist / (math.min(w, d) / 2)) / math.ln10;
    }
    final gainA = expectedGain(2, 2, 10, 10);
    final gainB = expectedGain(0.5, 0.5, 5, 5);
    double gainNow() => container.read(tuningStateProvider)[1]!.gainDb;

    container.read(activeRoomIdProvider.notifier).set('room1');
    await waitUntil(() => (gainNow() - gainA).abs() < 0.05, what: 'room1 스피커 기준 튜닝');
    container.read(activeRoomIdProvider.notifier).set('room2');
    await waitUntil(() => (gainNow() - gainB).abs() < 0.05, what: 'room2 스피커 기준 튜닝');
    container.read(activeRoomIdProvider.notifier).set('room1');
    await waitUntil(() => (gainNow() - gainA).abs() < 0.05, what: '다시 room1 기준 튜닝');
    container.dispose();
  });

  test('로우컷을 크로스오버에 맡기는 건 서브가 있는 방의 메인뿐이다', () async {
    // 방별 베이스 매니지먼트: 서브가 있는 방의 메인은 저역을 그 서브로 넘기므로 보호용
    // 로우컷을 끈다(크로스오버가 대신 자른다). 서브가 없는 방의 메인은 저역을 그대로
    // 내야 하므로 자기 저역 한계의 로우컷을 유지해야 한다.
    final container = await setUpContainer(); // room1: spk1(ch0)
    container.read(roomZoneProvider.notifier).addRoomZone(RoomZone(
          id: 'room2',
          x: 0,
          y: 0,
          width: 10,
          height: 10,
          color: 0x00FF00,
          physicalWidth: 10,
          physicalHeight: 10,
        ));
    final layout = container.read(speakerLayoutProvider.notifier);
    layout.addSpeaker(SpeakerNode(id: 'spk2', roomId: 'room2', x: 2.0, y: 2.0, channel: 1));
    layout.addSpeaker(SpeakerNode(id: 'sub1', roomId: 'room1', x: 5.0, y: 0.5, channel: 2));
    await waitUntil(() => container.read(tuningStateProvider)[3] != null, what: '서브 채널 계산');

    bool lowCutOn(int key) =>
        container.read(tuningStateProvider)[key]!.bandEnabled[kAutoEqSlotLowCut];
    expect(lowCutOn(1), isTrue, reason: '서브 지정 전에는 메인이 자기 로우컷을 가진다');
    expect(lowCutOn(2), isTrue);

    layout.setSubwoofer('sub1', true); // room1에만 서브
    await waitUntil(() => !lowCutOn(1), what: '서브가 생긴 방 메인의 로우컷 해제');
    expect(lowCutOn(2), isTrue, reason: '서브가 없는 방(room2)의 메인까지 로우컷이 풀렸다');
    container.dispose();
  });

  test('Lock Tuning으로 잠근 채널은 자동 계산이 건드리지 않는다', () async {
    final container = await setUpContainer();
    final notifier = container.read(tuningStateProvider.notifier);

    final locked = notifier.getTuning(1).copyWith(
          isTuningLocked: true,
          gainDb: -7.5,
          delay: 3.25,
        );
    notifier.saveTuning(1, locked);

    container.read(speakerLayoutProvider.notifier).updateSpeaker(
          SpeakerNode(id: 'spk1', roomId: 'room1', x: 8.5, y: 8.5, channel: 0),
        );
    await Future.delayed(const Duration(milliseconds: 250));

    final after = container.read(tuningStateProvider)[1]!;
    expect(after.gainDb, -7.5, reason: '잠근 채널의 게인이 덮어써졌다');
    expect(after.delay, 3.25, reason: '잠근 채널의 딜레이가 덮어써졌다');

    container.dispose();
  });

  test('RoomZone이 없어도 청사진 캔버스 기준으로 계산한다', () async {
    // 3D 뷰와 Auto-Aim은 방이 없으면 청사진 캔버스 치수로 폴백한다. 방을
    // 따로 만들지 않고 작업하는 사용자가 실제로 있는데, 예전에는 동기화만
    // `rooms.isEmpty`에서 통째로 빠져나가 EQ/게인/딜레이가 영영 갱신되지
    // 않았다(실기 보고: "스피커를 옮겼는데 아무 반응도 안 해").
    SharedPreferences.setMockInitialValues({});
    final container = ProviderContainer();

    container.read(speakerLayoutProvider.notifier).addSpeaker(
          SpeakerNode(id: 'spk1', x: 2.0, y: 2.0, channel: 0),
        );
    container.read(acousticSyncProvider);
    await Future.delayed(const Duration(milliseconds: 250));

    final tuning = container.read(tuningStateProvider)[1];
    expect(tuning, isNotNull,
        reason: '방이 없어도 청사진 기준으로 튜닝이 만들어져야 한다');

    // 배치 기반 EQ가 실제로 채워졌는지(자동 슬롯 중 하나라도 켜졌는지).
    expect(
      tuning!.bandEnabled.sublist(0, kAutoEqSlotCount).any((e) => e),
      isTrue,
      reason: '방이 없을 때 자동 EQ가 하나도 계산되지 않았다',
    );

    // 스피커를 옮기면 EQ가 따라 움직여야 한다.
    final beforeGains = List<double>.from(tuning.gains);
    container.read(speakerLayoutProvider.notifier).updateSpeaker(
          SpeakerNode(id: 'spk1', x: 0.3, y: 0.3, channel: 0),
        );
    await Future.delayed(const Duration(milliseconds: 250));
    expect(
      container.read(tuningStateProvider)[1]!.gains.sublist(0, kAutoEqSlotCount),
      isNot(equals(beforeGains.sublist(0, kAutoEqSlotCount))),
      reason: '방 없이 스피커를 옮겼는데 자동 EQ가 반응하지 않는다',
    );

    container.dispose();
  });

  test('딜레이와 게인이 타임 얼라인먼트 계산값과 일치한다', () async {
    final container = await setUpContainer();

    // 두 번째 스피커를 리스너에서 더 멀리 둔다. 정렬은 스피커가 둘 이상일
    // 때만 의미가 있고, 가장 먼 스피커가 기준(delay 0ms / gain 0dB)이 된다.
    container.read(speakerLayoutProvider.notifier).addSpeaker(
          SpeakerNode(id: 'spk2', roomId: 'room1', x: 9.5, y: 9.5, channel: 1),
        );
    await Future.delayed(const Duration(milliseconds: 250));

    final env = container.read(environmentStateProvider);
    final tunings = container.read(tuningStateProvider);

    const room = 10.0;
    const earLevel = 1.2; // RoomZone 기본값
    const heightZ = 3.5; // SpeakerNode 기본값

    double distanceOf(double sx, double sy) {
      final dx = sx - room / 2.0;
      final dy = sy - room / 2.0;
      const dz = heightZ - earLevel;
      return math.sqrt(dx * dx + dy * dy + dz * dz);
    }

    final dNear = distanceOf(2.0, 2.0); // ch1
    final dFar = distanceOf(9.5, 9.5); // ch2
    expect(dFar, greaterThan(dNear));

    final near = tunings[1];
    final far = tunings[2];
    expect(near, isNotNull);
    expect(far, isNotNull);

    // 기준:
    //  - 딜레이: 같은 방에서 **가장 먼 스피커**(추가 지연 최소화).
    //    방 모서리를 기준으로 쓰면 방 한가운데 스피커도 50ms 상한에 붙는다.
    //  - 게인: 청취 지점 -> 가장 가까운 벽(방 치수 고정).
    final double alignRef = dFar;
    const double gainRef = room / 2;

    expect(far!.delay, closeTo(0.0, 0.01),
        reason: '가장 먼 스피커가 기준이므로 딜레이가 0이어야 한다');
    expect(
      near!.delay,
      closeTo(((alignRef - dNear) / env.speedOfSound) * 1000.0, 0.01),
      reason: '딜레이가 (최원 스피커 거리 - 거리) / 음속과 다르다',
    );
    expect(near.delay, lessThan(50.0),
        reason: '10m 방에서 정렬 딜레이가 상한에 붙으면 기준이 잘못된 것이다');
    expect(near.gainDb, closeTo(20.0 * math.log(dNear / gainRef) / math.ln10, 0.01));
    expect(far.gainDb, closeTo(20.0 * math.log(dFar / gainRef) / math.ln10, 0.01));

    // 먼 쪽이 더 늦게 정렬되지 않고(딜레이는 적게), 더 크게 나가야 한다.
    expect(near.delay, greaterThan(far.delay));
    expect(near.gainDb, lessThan(far.gainDb));

    // 게인은 방 치수 기준이라 다른 스피커를 옮겨도 변하면 안 된다.
    // (딜레이는 정렬의 성질상 최원 스피커가 바뀌면 같이 바뀐다.)
    final ch1Before = container.read(tuningStateProvider)[1]!;
    container.read(speakerLayoutProvider.notifier).updateSpeaker(
          SpeakerNode(id: 'spk2', roomId: 'room1', x: 5.2, y: 5.2, channel: 1),
        );
    await Future.delayed(const Duration(milliseconds: 250));
    final ch1After = container.read(tuningStateProvider)[1]!;
    expect(ch1After.gainDb, ch1Before.gainDb,
        reason: 'ch2를 움직였는데 ch1 게인이 변했다 — 게인 기준이 '
            '다른 스피커에 의존하고 있다');

    container.dispose();
  });
}
