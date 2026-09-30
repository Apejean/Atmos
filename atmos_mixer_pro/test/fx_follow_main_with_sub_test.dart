import 'dart:math' as math;

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/acoustic_sync_provider.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/environment_state_provider.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/position_eq.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/settings/widgets/tuning_modal.dart';

/// 서브가 있는 방에서 메인(CH2)을 옮기면 FX가 그 자리를 따라가는지(사용자 요청, 2026-09-28).
///
/// 테마 1(15 x 20m, 천장 15m, 귀 1.6m)의 실제 배치: CH1 = 서브(8.74, 0.20, 2.61), CH2 = 메인.
/// CH2를 여러 자리로 옮기며 앱이 계산한 FX를 그 자리의 물리값과 대조한다.
/// - CH2 게인 = 20·log10(거리 / 7.5m) — 기준 거리는 방 짧은 변의 절반
/// - 시간 정렬: 같은 방에서 가장 먼 스피커가 0ms, 나머지는 (최원 거리 − 자기 거리) / 음속.
///   그래서 CH2가 움직이면 서브(CH1)의 지연도 따라 바뀐다
/// - 서브가 있는 방의 메인: 극성 반전 없음, EQ 로우컷 없음(크로스오버가 저역을 자른다)
/// - 서브: 20Hz 서브소닉 보호만
/// - 배치 EQ(경계면·룸 모드·반사·공기 흡음)는 자리마다 달라진다
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  Future<void> waitUntil(bool Function() done, String what) async {
    final deadline = DateTime.now().add(const Duration(seconds: 3));
    while (!done()) {
      if (DateTime.now().isAfter(deadline)) fail('$what이(가) 3초 안에 끝나지 않았다');
      await Future.delayed(const Duration(milliseconds: 10));
    }
    await Future.delayed(const Duration(milliseconds: 40));
  }

  const ear = (x: 7.5, y: 10.0, z: 1.6);
  const refDistance = 7.5;
  double distanceTo(({double x, double y, double z}) p) => math.sqrt(math.pow(p.x - ear.x, 2) +
      math.pow(p.y - ear.y, 2) +
      math.pow(p.z - ear.z, 2));

  test('서브가 있는 방에서 CH2를 옮기면 CH2와 서브의 FX가 자리를 따라간다', () async {
    SharedPreferences.setMockInitialValues({});
    final container = ProviderContainer();
    addTearDown(container.dispose);

    container.read(roomZoneProvider.notifier).addRoomZone(const RoomZone(
          id: 'theme1',
          x: 0,
          y: 0,
          width: 750,
          height: 1000,
          color: 0xFF1565C0,
          physicalWidth: 15,
          physicalHeight: 20,
          ceilingHeight: 15,
          earLevel: 1.6,
          absorptionCoeff: 0.0183,
        ));
    const subPos = (x: 8.74, y: 0.20, z: 2.61);
    final layout = container.read(speakerLayoutProvider.notifier);
    layout.addSpeaker(SpeakerNode(
        id: 'sub', roomId: 'theme1', x: subPos.x, y: subPos.y, heightZ: subPos.z, channel: 0));
    layout.addSpeaker(
        SpeakerNode(id: 'main', roomId: 'theme1', x: 13.03, y: 15.03, heightZ: 2.44, channel: 1));
    layout.setSubwoofer('sub', true);
    container.read(acousticSyncProvider);

    final c = container.read(environmentStateProvider).speedOfSound;
    final dSub = distanceTo(subPos);
    final seenEq = <String>{};

    for (final pos in const [
      (x: 13.03, y: 15.03, z: 2.44), // 지금 자리
      (x: 7.5, y: 13.0, z: 2.0), // 청취 지점 바로 앞(서브보다 가까움)
      (x: 1.0, y: 19.0, z: 3.0), // 먼 모서리
      (x: 4.56, y: 14.38, z: 14.75), // 천장 바로 아래(서브보다 멂)
    ]) {
      layout.updateSpeaker(SpeakerNode(
          id: 'main', roomId: 'theme1', x: pos.x, y: pos.y, heightZ: pos.z, channel: 1));
      final dMain = distanceTo(pos);
      final dMax = math.max(dSub, dMain);
      final wantGain = (20 * math.log(dMain / refDistance) / math.ln10).clamp(-24.0, 12.0);
      final wantMainDelay = ((dMax - dMain) / c * 1000).clamp(0.0, 50.0);
      final wantSubDelay = ((dMax - dSub) / c * 1000).clamp(0.0, 50.0);

      await waitUntil(() {
        final t = container.read(tuningStateProvider)[2];
        return t != null && (t.gainDb - wantGain).abs() < 0.02;
      }, 'CH2 $pos 자동 계산');
      final main = container.read(tuningStateProvider)[2]!;
      final sub = container.read(tuningStateProvider)[1]!;

      expect(main.gainDb, closeTo(wantGain, 0.02), reason: 'CH2 $pos 게인');
      expect(main.delay, closeTo(wantMainDelay, 0.02), reason: 'CH2 $pos 시간 정렬');
      expect(sub.delay, closeTo(wantSubDelay, 0.02),
          reason: 'CH2가 $pos로 옮기면 서브의 시간 정렬도 따라가야 한다');
      expect(main.phaseInvert, isFalse, reason: '서브가 있는 방의 메인은 극성을 뒤집지 않는다');
      expect(sub.phaseInvert, isFalse);
      expect(main.bandEnabled[kAutoEqSlotLowCut], isFalse,
          reason: '크로스오버가 메인 저역을 자르므로 EQ 로우컷을 겹치지 않는다');
      expect(sub.bandEnabled[kAutoEqSlotLowCut], isTrue);
      expect(sub.freqs[kAutoEqSlotLowCut], closeTo(kSubsonicHz, 0.01));
      seenEq.add(main.gains.take(kAutoEqSlotCount).join(','));
    }
    expect(seenEq.length, 4, reason: 'CH2 자리를 바꿨는데 배치 EQ가 같은 값으로 남았다');
  });
}
