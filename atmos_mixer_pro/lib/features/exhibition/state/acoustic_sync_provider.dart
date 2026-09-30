import 'dart:async';
import 'dart:math' as math;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter/foundation.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/environment_state_provider.dart';
import 'package:atmos_mixer_pro/features/settings/widgets/tuning_modal.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/position_eq.dart';

import 'package:atmos_mixer_pro/features/exhibition/state/bass_management_provider.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/field_geometry.dart';

class AcousticSyncProvider extends Notifier<void> {
  Timer? _throttleTimer;

  @override
  void build() {
    // Watch speaker and room states to trigger recalculation
    ref.listen(speakerLayoutProvider, (previous, next) {
      _scheduleRecalculation();
    });
    ref.listen(roomZoneProvider, (previous, next) {
      // 방 크기나 귀높이가 바뀌면 청취 지점이 움직이므로 자동 조준을 갱신한다.
      ref.read(speakerLayoutProvider.notifier).reaimAutoSpeakers();
      _scheduleRecalculation();
    });
    ref.listen(environmentStateProvider, (previous, next) {
      _scheduleRecalculation();
    });
    ref.listen(bassManagementProvider, (previous, next) {
      _scheduleRecalculation();
    });
    // 같은 채널을 여러 방에 쓴 경우 보고 있는 방의 스피커가 그 채널을 대표한다
    // (speaker_node.dart channelRepresentatives). 방을 바꾸면 그 채널 튜닝도 다시 계산한다.
    ref.listen(activeRoomIdProvider, (previous, next) {
      _scheduleRecalculation();
    });
    ref.listen(blueprintProvider, (previous, next) {
      // 방을 따로 만들지 않고 청사진 캔버스만 쓰는 경우, 캔버스 치수가
      // 청취 지점을 결정하므로 여기서도 다시 계산해야 한다.
      _scheduleRecalculation();
    });
  }

  void _scheduleRecalculation() {
    if (_throttleTimer?.isActive ?? false) return;
    _throttleTimer = Timer(const Duration(milliseconds: 16), () {
      _recalculateWhenTuningLoaded();
    });
  }

  /// 저장된 채널 튜닝(잠금·수동 밴드 표시 포함)을 불러온 뒤에 계산한다.
  ///
  /// 예전에는 앱을 켤 때 이 계산이 저장값 불러오기보다 먼저 끝날 수 있었다. 그러면 불러오기가
  /// 계산 결과를 옛 저장값으로 통째로 덮어쓰고 엔진에도 옛 값을 다시 보내서, 배치·규칙이 바뀐
  /// 뒤 앱을 다시 켜도 스피커를 한 번 움직이기 전까지 옛 튜닝이 걸렸다. 잠근 채널도 불러오기
  /// 전에는 잠금 표시를 몰라 잠깐 덮어썼다.
  Future<void> _recalculateWhenTuningLoaded() async {
    try {
      await ref.read(tuningStateProvider.notifier).ensureLoaded();
    } catch (e) {
      // 불러오기가 실패해도 자동 계산은 멈추지 않는다(예전 동작).
      debugPrint('채널 튜닝 불러오기 실패, 그대로 계산: $e');
    }
    if (!ref.mounted) return;
    _recalculateAndSync();
  }

  void _recalculateAndSync() {
    final rooms = ref.read(roomZoneProvider);
    // 채널마다 한 스피커만 계산한다. 같은 채널을 여러 방에 쓰면 예전에는 목록의 마지막
    // 스피커 값이 이겨서(엔진 좌표는 첫 스피커) 다른 방 기준 튜닝이 걸렸다.
    final speakers = channelRepresentatives(
      ref.read(speakerLayoutProvider),
      activeRoomOf(rooms, ref.read(activeRoomIdProvider))?.id,
    ).values.toList();
    final tuningNotifier = ref.read(tuningStateProvider.notifier);

    // SpeakerNode.x/y는 **이미 미터**다(생성 시 `roomWidth * 0.25`, 3D 룸
    // 드래그도 미터로 되돌려준다). 예전에는 캔버스 픽셀로 오해해 scale(50)로
    // 나눴는데, RoomZone.physical*/earLevel과 단위가 어긋나 거리·각도 계산이
    // 전부 틀어졌다. 변환하지 않는다.
    double xMeters(SpeakerNode n) => n.x;
    double yMeters(SpeakerNode n) => n.y;

    bool changed = false;

    // RoomZone을 하나도 만들지 않고 청사진 캔버스 위에서만 작업하는 경우가
    // 있다(3D 뷰와 Auto-Aim이 그 상황을 캔버스 치수로 폴백해 지원한다).
    // 예전에는 여기서 `if (rooms.isEmpty) continue`로 통째로 빠져나가서,
    // 방을 안 만든 사용자는 스피커를 아무리 옮겨도 EQ/게인/딜레이가 전혀
    // 갱신되지 않았다(실기 보고: "스피커를 옮겼는데 아무 반응도 안 해").
    final bp = ref.read(blueprintProvider);

    for (final speaker in speakers) {
      // 방·청취 지점은 헤드폰 현장 물리 밴드(spatial_sync)와 같은 규칙으로 정한다.
      final g = fieldGeometryFor(speaker, rooms, bp);
      final double roomWidth = g.roomWidth;
      final double roomDepth = g.roomDepth;
      final double roomCeiling = g.ceilingHeight;
      final env = ref.read(environmentStateProvider);

      final chKey = speaker.channel + 1;
      final currentTuning = tuningNotifier.getTuning(chKey);

      // 1. Lock 튜닝 채널은 무시 (덮어쓰기 방지)
      if (currentTuning.isTuningLocked) {
        continue;
      }

      // 리스너 좌표 (방 정중앙)
      final double xEar = g.listenerX;
      final double yEar = g.listenerY;
      final double zEar = g.listenerZ;

      // 현재 스피커와 리스너 간의 거리 계산
      final double dx = xMeters(speaker) - xEar;
      final double dy = yMeters(speaker) - yEar;
      final double dz = speaker.heightZ - zEar;
      final double distance = math.sqrt(dx * dx + dy * dy + dz * dz);

      // 기준 거리는 **방 치수만으로** 정한다. 다른 스피커 위치에 의존하면
      // 안 된다.
      //
      // 예전에는 딜레이 기준을 "가장 먼 스피커", 게인 기준을 "평균 거리"로
      // 잡았다. 정렬 자체는 맞지만, 스피커 하나를 움직이면 기준이 따라
      // 움직여서 **나머지 채널의 게인과 딜레이가 전부 흔들렸다**. 실기에서
      // ch2를 움직였는데 ch1 소리가 울렁거린 게 이것이다.
      //
      // 방 기준이면 각 채널은 자기 거리에만 반응한다. 모든 채널에 같은
      // 상수를 더하는 것뿐이라 정렬 결과(동시 도착)는 그대로다.
      //
      // - 딜레이 기준: 청취 지점에서 방의 가장 먼 모서리까지. 방 안 어디에
      //   놓아도 딜레이가 음수가 되지 않는다.
      // - 게인 기준: 청취 지점에서 가장 가까운 벽까지. 벽쯤에 놓은 스피커가
      //   0dB고, 더 멀면 +, 더 가까우면 -가 된다.
      // 딜레이 기준은 **같은 방에서 가장 먼 스피커**다.
      //
      // 정렬이란 "가장 늦게 도착하는 소리에 나머지를 맞추는 것"이라, 기준이
      // 실제 최원 스피커여야 추가 지연이 최소가 된다. 한때 방의 가장 먼
      // 모서리를 기준으로 삼아 봤는데(채널 간 독립성 때문에), 스피커가 방
      // 한가운데 있어도 모서리까지의 거리만큼 기다리게 되어 50ms 상한에
      // 붙어버렸다(실기 보고: "가까이 놓아도 50ms").
      //
      // 최원 스피커를 기준으로 쓰면 다른 스피커를 옮길 때 이 값도 바뀌는데,
      // 그건 정렬의 성질상 피할 수 없다. 대신 드래그 **중**에는 딜레이를
      // 고정해 두어(아래 dragging 분기) 움직이는 내내 출렁이지는 않는다.
      double maxDistance = 0.0;
      for (final spk in speakers) {
        if (spk.roomId != speaker.roomId) continue;
        final sdx = xMeters(spk) - xEar;
        final sdy = yMeters(spk) - yEar;
        final sdz = spk.heightZ - zEar;
        final d = math.sqrt(sdx * sdx + sdy * sdy + sdz * sdz);
        if (d > maxDistance) maxDistance = d;
      }
      final double alignRefDistance =
          maxDistance > 0.0 ? maxDistance : distance;

      // 게인 기준은 방 치수로 고정한다(청취 지점 -> 가장 가까운 벽).
      // 게인은 정렬처럼 시스템 전체 기준이 필요한 값이 아니라 단순 정규화라,
      // 고정해 두면 다른 스피커를 옮겨도 이 채널 게인이 흔들리지 않는다.
      // 헤드폰 미리듣기는 같은 기준으로 거리 감쇠를 흉내 내서 이 게인을 상쇄한다 —
      // Rust acoustic::gain_reference_distance와 같은 식이어야 한다.
      final double gainRefDistance =
          math.max(math.min(roomWidth, roomDepth) / 2.0, 0.5);
      // roomCeiling은 자동 EQ(경계면 반사)에서 쓴다.
      
      // 1. 타임 얼라인먼트 딜레이 (ms) - 현재 온도 기준 동적 음속
      //
      // 방의 가장 먼 모서리를 기준(0ms)으로 삼고, 그보다 가까운 스피커를
      // 그만큼 **늦춰서** 모든 소리가 청취 지점에 동시에 도착하게 한다:
      // delay = (d_ref - d) / c.
      //
      // 예전에는 절대 비행 시간(d / c)을 그대로 넣었다. "스피커가 한 대일 때도
      // 숫자가 움직이게" 하려던 것인데, 이건 정렬이 아니라 정반대다. 이미
      // 17ms 늦게 도착하는 먼 스피커를 17ms 더 늦춰서 오차를 두 배로 키운다.
      // 스피커가 한 대면 정렬할 대상이 없으므로 0ms가 맞는 값이다.
      final double speedOfSound = env.speedOfSound;
      double delayMs =
          ((alignRefDistance - distance) / speedOfSound) * 1000.0;
      delayMs = delayMs.clamp(0.0, 50.0);

      // 2. 거리 감쇠 보정 게인 (Inverse Square Law)
      //
      // 청취 지점에서의 SPL을 맞추려면 전기 게인이 거리에 비례해야 한다:
      // gain = 20*log10(d / d_ref). 기준보다 멀면 +, 가까우면 -다.
      //
      // 예전 식 20*log10(1 / d)는 부호가 통째로 뒤집혀 있었다. 먼 스피커는
      // 거리 때문에 이미 작게 들리는데 거기서 더 깎아서 오차를 키웠다.
      double gainDb =
          20.0 * math.log(math.max(distance, 0.01) / gainRefDistance) /
              math.ln10;
      
      // LFE로 지정한 채널에 +10dB를 자동으로 더하지 않는다. 예전에는 그렇게 했는데,
      // 그 채널이 풀레인지 프로그램을 내고 있으면 중·고역까지 10dB 커져 날카롭게
      // 들렸다. 표준에서 +10dB는 .1(LFE) 트랙에만 걸리는 값이고, 서브 레벨은 출력단이나
      // 하드웨어에서 맞춘다. 소프트웨어로 필요하면 베이스 매니지먼트의 LFE +10dB
      // 토글(120Hz 로우패스 이후에만 적용, rust mixer.rs)을 쓴다.
      gainDb = gainDb.clamp(-24.0, 12.0);

      // 베이스 매니지먼트에서의 역할(방별). 서브우퍼면 서브용 자동 FX를, 자기 방에
      // 서브가 있는 메인이면 로우컷을 크로스오버에 맡긴다(position_eq.dart 참고).
      // 서브가 없는 방의 메인은 저역을 그대로 내므로 자기 저역 한계의 로우컷을 유지한다
      // (엔진도 그 방 저역은 다른 방 서브로 보내지 않는다 — rust bass_route.rs).
      final bm = ref.read(bassManagementProvider);
      final bool isSubwoofer = speaker.isSubwoofer;
      final bool roomHasSub =
          speakers.any((s) => s.isSubwoofer && s.roomId == speaker.roomId);
      final double? bassManagedCrossoverHz =
          (roomHasSub && !isSubwoofer) ? bm.crossoverFreq : null;

      // 3. 스피커 배치에서 유도되는 EQ 자동 계산
      // 로우컷 / 경계면 저음 / 바닥 반사 딥 / 공기 흡음 / 오프액시스까지
      // 밴드 on-off, 쉐입 타입, 주파수, 게인, Q를 전부 계산한다. 계산식은
      // position_eq.dart에 순수 함수로 분리해 두었다(테스트 대상).
      final autoBands = computeAutoEqBands(
        speakerX: xMeters(speaker),
        speakerY: yMeters(speaker),
        speakerZ: speaker.heightZ,
        listenerX: xEar,
        listenerY: yEar,
        listenerZ: zEar,
        roomWidth: roomWidth,
        roomDepth: roomDepth,
        ceilingHeight: roomCeiling,
        yawDeg: speaker.rotation,
        autoAim: speaker.autoAim,
        dispersionAngleDeg: speaker.dispersionAngle,
        dispersionAngleVDeg: speaker.dispersionAngleV,
        pitchTiltDeg: speaker.pitchTilt,
        lowCutHz: speaker.lowCutHz,
        speedOfSound: speedOfSound,
        absorptionCoeff: g.absorption,
        isSubwoofer: isSubwoofer,
        bassManagedCrossoverHz: bassManagedCrossoverHz,
      );

      // 4. 마주보는 스피커 간 파형 상쇄 방지를 위한 위상 반전 자동화
      // 단순히 뒷벽에 붙었는지가 아니라, "청취자(yEar) 기준 뒤편에 배치되어
      // 전면 스피커와 마주보게 되는가"가 핵심이다. 1.0m 이상 뒤면 서라운드
      // 스피커로 보고 저역 상쇄를 막기 위해 180도 뒤집는다.
      //
      // 서브우퍼는 예외다. 서브의 극성은 위치가 아니라 크로스오버 지점에서 메인과
      // 합쳐지는 방향으로 맞춰야 하고(측정·청음으로 정한다), 위치만 보고 뒤집으면
      // 크로스오버 대역이 상쇄될 수 있다.
      //
      // 서브가 있는 방의 메인도 뒤집지 않는다. 그 방의 저역은 서브가 내므로 앞·뒤 메인이
      // 저역에서 부딪칠 일이 없고, 메인만 뒤집으면 크로스오버 지점에서 서브와 반대 극성이
      // 되어 그 대역이 지워진다(엔진은 메인의 저역을 극성 보정 전에 갈라 서브로 보낸다).
      final bool phaseInvert =
          !isSubwoofer && !roomHasSub && yMeters(speaker) > yEar + 1.0;

      // 드래그 중에는 시간정렬 딜레이를 그대로 둔다.
      //
      // 딜레이 탭을 매 프레임 옮기면, 아무리 크로스페이드를 걸어도 페이드가
      // 끝나기 전에 다음 목표가 도착해 계속 재시작된다. 그 결과가 실기에서
      // 들린 "테이프 스톱/스타트" 소리다. 정렬 딜레이는 움직이는 음원이
      // 아니라 정지한 스피커의 보정값이므로, 드래그를 놓았을 때 한 번만
      // 깔끔하게 바꾸는 게 맞다. 게인·EQ·팬은 드래그 중에도 계속 따라간다.
      final bool dragging =
          ref.read(speakerLayoutProvider.notifier).isDragging;

      final base = currentTuning.copyWith(
        delay: dragging ? currentTuning.delay : delayMs,
        phaseInvert: phaseInvert,
        gainDb: gainDb,
      );

      // saveTuning이 아니라 applyAutoTuning을 쓴다. saveTuning은 사람이 고친
      // 밴드를 자동에서 떼어내는 경로라, 자동 계산이 그걸 부르면 자기가 쓴
      // 값을 보고 "사람이 손댔다"고 판단해 버린다.
      final didChange = tuningNotifier.applyAutoTuning(chKey, base, autoBands);
      if (didChange) {
        debugPrint(
          'AcousticSync: ch $chKey -> delay ${delayMs.toStringAsFixed(2)}ms, '
          'gain ${gainDb.toStringAsFixed(2)}dB, phaseInvert $phaseInvert, '
          'eq ${autoBands.where((b) => b.enabled).join(' | ')}',
        );
        changed = true;
      }
    }

    if (changed) {
      tuningNotifier.applyAllToBackend();
      debugPrint('AcousticSync: applyAllToBackend called.');
    }
  }
}

final acousticSyncProvider = NotifierProvider<AcousticSyncProvider, void>(AcousticSyncProvider.new);
