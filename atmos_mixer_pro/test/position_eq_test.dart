import 'dart:math' as math;
import 'package:flutter_test/flutter_test.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/position_eq.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';

const double kC = 343.0;

/// 기본 배치: 10m x 10m x 3m 방, 청취자는 정중앙 귀높이 1.2m.
List<AutoEqBand> bandsFor({
  double x = 5.0,
  double y = 3.0,
  double z = 2.0,
  double yaw = 0.0,
  double dispersion = 90.0,
  double dispersionV = 180.0,
  double pitch = 0.0,
  bool autoAim = false,
  double lowCut = 80.0,
  double absorption = 0.3,
  double ceiling = 3.0,
  bool isSubwoofer = false,
  double? bassManagedCrossover,
}) {
  return computeAutoEqBands(
    isSubwoofer: isSubwoofer,
    bassManagedCrossoverHz: bassManagedCrossover,
    speakerX: x,
    speakerY: y,
    speakerZ: z,
    listenerX: 5.0,
    listenerY: 5.0,
    listenerZ: 1.2,
    roomWidth: 10.0,
    roomDepth: 10.0,
    ceilingHeight: ceiling,
    yawDeg: yaw,
    autoAim: autoAim,
    dispersionAngleDeg: dispersion,
    dispersionAngleVDeg: dispersionV,
    pitchTiltDeg: pitch,
    lowCutHz: lowCut,
    speedOfSound: kC,
    absorptionCoeff: absorption,
  );
}

AutoEqBand slot(List<AutoEqBand> bands, int s) =>
    bands.firstWhere((b) => b.slot == s);

/// [bandsFor]와 같은 배치의 헤드폰 미리듣기 현장 물리 밴드.
List<AutoEqBand> physicsFor({
  double x = 5.0,
  double y = 3.0,
  double z = 2.0,
  double yaw = 0.0,
  double dispersion = 90.0,
  double dispersionV = 180.0,
  double pitch = 0.0,
  bool autoAim = false,
  double absorption = 0.3,
  double ceiling = 3.0,
  bool isSubwoofer = false,
}) {
  return computeFieldPhysicsBands(
    isSubwoofer: isSubwoofer,
    speakerX: x,
    speakerY: y,
    speakerZ: z,
    listenerX: 5.0,
    listenerY: 5.0,
    listenerZ: 1.2,
    roomWidth: 10.0,
    roomDepth: 10.0,
    ceilingHeight: ceiling,
    yawDeg: yaw,
    autoAim: autoAim,
    dispersionAngleDeg: dispersion,
    dispersionAngleVDeg: dispersionV,
    pitchTiltDeg: pitch,
    speedOfSound: kC,
    absorptionCoeff: absorption,
  );
}

void main() {
  test('8밴드를 모두 자동 계산이 채우고 슬롯은 역할별로 고정된다', () {
    final bands = bandsFor();
    expect(bands.length, kAutoEqSlotCount);
    expect(kAutoEqSlotCount, 8);
    expect(bands.map((b) => b.slot).toList(), [0, 1, 2, 3, 4, 5, 6, 7]);

    // 배치를 크게 바꿔도 역할이 슬롯을 옮겨다니지 않아야 한다
    // (예전 "빈 밴드 찾아 끼우기" 방식의 실패 모드).
    final moved = bandsFor(x: 0.2, y: 9.5, z: 2.8, yaw: 170.0);
    expect(moved.map((b) => b.slot).toList(), [0, 1, 2, 3, 4, 5, 6, 7]);
    expect(slot(moved, kAutoEqSlotBoundary).type, EqType.lowShelf);
    expect(slot(moved, kAutoEqSlotOffAxis).type, EqType.highShelf);
  });

  test('로우컷은 스피커 저역 한계 그대로 12dB/oct로 건다 (경계면에서 올리지 않는다)', () {
    // 예전에는 경계면에 가까우면 차단 주파수를 최대 1.5배 올리고 기울기를 24dB/oct까지
    // 세웠다. 벽에서 0.2m인 스피커는 115Hz·24dB/oct가 되어 저역이 통째로 빠졌다
    // (실기 보고: "EQ에서 로우컷이 많이 된다"). 경계면 부풂은 슬롯 1 로우셸프가
    // 이미 보정하므로 이중 보정이었다.
    final free = slot(bandsFor(x: 5.0, y: 5.0, z: 1.5), kAutoEqSlotLowCut);
    final wall = slot(bandsFor(x: 0.8, y: 5.0, z: 1.5), kAutoEqSlotLowCut);
    final corner = slot(bandsFor(x: 0.2, y: 0.2, z: 0.3), kAutoEqSlotLowCut);
    for (final b in [free, wall, corner]) {
      expect(b.type, EqType.lowCut);
      expect(b.freq, closeTo(80.0, 0.01), reason: '경계면 거리와 무관하게 저역 한계 그대로');
      expect(b.slopeDbPerOct, 12);
    }
    // 경계면 부풂은 로우셸프가 맡는다.
    expect(slot(bandsFor(x: 0.2, y: 0.2, z: 0.3), kAutoEqSlotBoundary).enabled, isTrue);
  });

  test('경계면 쉘프 주파수는 가장 가까운 면까지 거리로 계산된다 (f = c / 4d)', () {
    // 예전에는 150Hz 고정이라 스피커를 옮겨도 주파수가 그대로였다.
    final half = slot(bandsFor(x: 0.5, y: 5.0, z: 1.5), kAutoEqSlotBoundary);
    expect(half.freq, closeTo(kC / (4 * 0.5), 1.0));

    final farther = slot(bandsFor(x: 1.5, y: 5.0, z: 1.5), kAutoEqSlotBoundary);
    expect(farther.freq, closeTo(kC / (4 * 1.5), 1.0));
    expect(farther.freq, lessThan(half.freq));
    expect(half.gain, lessThan(farther.gain), reason: '벽에 붙을수록 더 깎아야 한다');

    // 모든 경계가 멀면 보정이 사라진다.
    final free = computeAutoEqBands(
      speakerX: 20.0, speakerY: 20.0, speakerZ: 15.0,
      listenerX: 20.0, listenerY: 24.0, listenerZ: 15.0,
      roomWidth: 40.0, roomDepth: 40.0, ceilingHeight: 30.0,
      yawDeg: 0.0, autoAim: false, dispersionAngleDeg: 90.0,
      dispersionAngleVDeg: 180.0, pitchTiltDeg: 0.0,
      lowCutHz: 80.0, speedOfSound: kC,
    );
    expect(slot(free, kAutoEqSlotBoundary).enabled, isFalse);
  });

  test('반사 보정은 상쇄 지점이 아니라 보강 피크를 겨냥한다 (f = c / 경로차)', () {
    // 이미 반사로 소리가 빠진 골짜기를 EQ로 또 깎으면 더 깊어질 뿐이다.
    // 보정할 수 있는 건 솟아오른 피크 쪽이다.
    const double sx = 5.0, sy = 3.0, sz = 2.0;
    const double lz = 1.2;
    final double horiz = (5.0 - sy).abs();
    final double direct = math.sqrt(horiz * horiz + (sz - lz) * (sz - lz));
    final double reflected =
        math.sqrt(horiz * horiz + (sz + lz) * (sz + lz));
    final double pathDiff = reflected - direct;

    final band = slot(bandsFor(x: sx, y: sy, z: sz), kAutoEqSlotFloorBounce);
    expect(band.enabled, isTrue);
    expect(band.type, EqType.bell);
    expect(band.gain, lessThan(0.0));
    expect(band.freq, closeTo(kC / pathDiff, 2.0), reason: '보강 피크를 겨냥해야 한다');
    expect((band.freq - kC / (2 * pathDiff)).abs(), greaterThan(20.0),
        reason: '상쇄(null) 주파수를 깎으면 안 된다');
  });

  test('반사 보정량은 흡음이 높을수록 작아진다', () {
    final live = slot(bandsFor(absorption: 0.1), kAutoEqSlotFloorBounce);
    final dead = slot(bandsFor(absorption: 0.8), kAutoEqSlotFloorBounce);
    expect(live.gain, lessThan(dead.gain),
        reason: '흡음이 적은 방일수록 반사가 세니 더 깎아야 한다');
    expect(live.q, greaterThan(dead.q),
        reason: '반사가 셀수록 피크가 좁고 뚜렷하다');
  });

  test('천장·측벽 반사도 각각 밴드를 가진다', () {
    // 천장이 높은 방에서 매달린 스피커: 천장 반사 경로차가 유효 구간에 들어온다.
    final hung = computeAutoEqBands(
      speakerX: 5.0, speakerY: 2.0, speakerZ: 3.0,
      listenerX: 5.0, listenerY: 5.0, listenerZ: 1.2,
      roomWidth: 10.0, roomDepth: 10.0, ceilingHeight: 5.0,
      yawDeg: 0.0, autoAim: true, dispersionAngleDeg: 90.0,
      dispersionAngleVDeg: 180.0, pitchTiltDeg: 0.0,
      lowCutHz: 80.0, speedOfSound: kC,
    );
    expect(slot(hung, kAutoEqSlotCeilingBounce).enabled, isTrue);
    expect(slot(hung, kAutoEqSlotCeilingBounce).type, EqType.bell);

    final nearWall = slot(bandsFor(x: 0.6, y: 3.0), kAutoEqSlotWallBounce);
    expect(nearWall.enabled, isTrue);
    expect(nearWall.gain, lessThan(0.0));
  });

  test('룸 모드 보정은 청취 지점에서 실제로 솟는 모드를 깎는다 (방 한가운데는 홀수 차수의 마디)', () {
    // 청취자는 10x10x3m 방 정중앙(5, 5, 1.2). 가로·세로 1차 모드(17Hz)는 20Hz 아래이고,
    // 2차 모드(c/L = 34Hz)는 한가운데가 배(가장 큼)다. 스피커도 한가운데면 가로·세로 2차가
    // 같은 주파수로 겹쳐(정사각형 방) 가장 세게 들린다.
    final center = slot(bandsFor(x: 5.0, y: 5.0, z: 1.5), kAutoEqSlotRoomMode);
    expect(center.enabled, isTrue);
    expect(center.type, EqType.bell);
    expect(center.freq, closeTo(kC / 10.0, 1.0));
    expect(center.gain, lessThan(-5.0), reason: '겹친 두 모드라 한 모드보다 세다');

    // 흡음이 많은 방일수록 모드가 덜 울리므로 덜 깎고 Q도 낮다.
    final dead = slot(bandsFor(x: 5.0, y: 5.0, z: 1.5, absorption: 0.8), kAutoEqSlotRoomMode);
    expect(dead.gain, greaterThan(center.gain));
    expect(dead.q, lessThan(center.q));
  });

  test('청취 지점이 마디인 모드는 깎지 않는다 (예전 모델은 없는 공진을 깎았다)', () {
    // 5x5x5m 방(현장 '2'번 방), 청취자 정중앙 귀 높이. 스피커는 뒤 벽 근처(0.57, 4.8, 1.8).
    // 예전 모델은 음원 쪽만 보고 세로 1차 모드(34Hz)를 깎았는데, 방 한가운데는 그 모드의
    // 마디라 실제로는 거의 안 들린다 — 깎으면 그 주파수에 구멍이 난다.
    // 청취 지점에서 실제로 솟는 건 2차 모드(69Hz, 가로·세로가 겹침)다.
    final bands = computeAutoEqBands(
      speakerX: 0.57, speakerY: 4.8, speakerZ: 1.8,
      listenerX: 2.5, listenerY: 2.5, listenerZ: 1.2,
      roomWidth: 5.0, roomDepth: 5.0, ceilingHeight: 5.0,
      yawDeg: 0.0, autoAim: true, dispersionAngleDeg: 90.0,
      dispersionAngleVDeg: 90.0, pitchTiltDeg: 0.0,
      lowCutHz: 45.0, speedOfSound: kC,
    );
    final mode = slot(bands, kAutoEqSlotRoomMode);
    expect(mode.enabled, isTrue);
    expect(mode.freq, closeTo(kC / 5.0, 1.0), reason: '2차 모드(69Hz)를 깎아야 한다');
    expect((mode.freq - kC / 10.0).abs(), greaterThan(10.0),
        reason: '청취 지점이 마디인 1차 모드(34Hz)를 깎으면 안 된다');
  });

  test('공기 흡음 보상은 5m 안에서는 꺼지고 그 밖에서 1m당 +0.5dB', () {
    expect(slot(bandsFor(y: 3.0), kAutoEqSlotAirAbsorption).enabled, isFalse);
    final far = slot(
      bandsFor(x: 5.0, y: 14.0, z: 1.2),
      kAutoEqSlotAirAbsorption,
    );
    expect(far.enabled, isTrue);
    expect(far.type, EqType.highShelf);
    expect(far.freq, 10000.0);
    expect(far.gain, closeTo((9.0 - 5.0) * 0.5, 0.01));
  });

  test('오프액시스 고역 감쇠는 자동 EQ로 걸지 않는다 (현장에서 이중 감쇠)', () {
    // 커버리지 콘 밖에서 고역이 빠지는 건 스피커가 실제로 만드는 현상이다. EQ로 또 깎으면
    // 현장에서 두 번 깎인다. 헤드폰 미리듣기의 물리 밴드(지향성)로만 흉내 낸다.
    for (final yaw in [0.0, 60.0, 180.0]) {
      final b = slot(bandsFor(x: 5.0, y: 2.0, yaw: yaw), kAutoEqSlotOffAxis);
      expect(b.enabled, isFalse, reason: 'yaw $yaw°에서 자동 EQ가 고역을 깎았다');
      expect(b.type, EqType.highShelf, reason: '슬롯 자리는 그대로 둔다');
    }
  });

  test('헤드폰 지향성: 커버리지 콘 안이면 없고, 밖이면 고역이 빠진다', () {
    final onAxis = physicsFor(x: 5.0, y: 2.0, yaw: 0.0)[kPhysicsSlotDirectivity];
    expect(onAxis.enabled, isFalse);

    final backwards = physicsFor(x: 5.0, y: 2.0, yaw: 180.0)[kPhysicsSlotDirectivity];
    expect(backwards.enabled, isTrue);
    expect(backwards.type, EqType.highShelf);
    expect(backwards.gain, -18.0); // (180-45)*0.2 = 27 -> 상한 18

    final justOutside = physicsFor(x: 5.0, y: 2.0, yaw: 60.0)[kPhysicsSlotDirectivity];
    expect(justOutside.gain, closeTo(-3.0, 0.05));

    final wide = physicsFor(x: 5.0, y: 2.0, yaw: 60.0, dispersion: 160.0)[kPhysicsSlotDirectivity];
    expect(wide.enabled, isFalse);
  });

  test('헤드폰 지향성: 조준하지 않은 스피커는 청취자를 향한 것으로 본다', () {
    expect(physicsFor(x: 5.0, y: 8.0, autoAim: true)[kPhysicsSlotDirectivity].enabled, isFalse);
    expect(physicsFor(x: 5.0, y: 8.0, autoAim: false)[kPhysicsSlotDirectivity].enabled, isTrue);
  });

  test('헤드폰 지향성: 수직 분산각 밖이면 고역이 빠지고, 아래로 조준하면 회복된다', () {
    final flat = physicsFor(x: 5.0, y: 2.0, z: 2.8, dispersionV: 40.0, pitch: 0.0);
    expect(flat[kPhysicsSlotDirectivity].enabled, isTrue);
    // pitch 음수 = 아래로 조준(Auto-Aim 버튼과 3D 씬의 규약).
    final elevation = math.atan2(1.2 - 2.8, 3.0) * 180.0 / math.pi;
    final aimedDown = physicsFor(x: 5.0, y: 2.0, z: 2.8, dispersionV: 40.0, pitch: elevation);
    expect(aimedDown[kPhysicsSlotDirectivity].enabled, isFalse,
        reason: '청취자를 정조준했으면 감쇠가 없어야 한다');
  });

  test('자동 조준은 청취자를 정확히 향하는 yaw/pitch를 만든다', () {
    final north = aimAtListener(
      speakerX: 5.0, speakerY: 9.0, speakerZ: 1.2,
      listenerX: 5.0, listenerY: 5.0, listenerZ: 1.2,
    );
    expect(north.yawDeg.abs(), closeTo(180.0, 0.01));
    expect(north.pitchDeg, closeTo(0.0, 0.01));

    final east = aimAtListener(
      speakerX: 9.0, speakerY: 5.0, speakerZ: 1.2,
      listenerX: 5.0, listenerY: 5.0, listenerZ: 1.2,
    );
    expect(east.yawDeg, closeTo(-90.0, 0.01));

    // pitch 규약: **음수가 아래**. 원래 있던 'Auto-Aim to Listener' 버튼이
    // atan2(earLevel - heightZ, 수평거리)를 쓰고 3D 씬이 rotation.x = -pitchRad로
    // 그린다. 부호를 뒤집으면 매달린 스피커가 천장을 본다.
    final hung = aimAtListener(
      speakerX: 5.0, speakerY: 2.0, speakerZ: 4.2,
      listenerX: 5.0, listenerY: 5.0, listenerZ: 1.2,
    );
    expect(hung.pitchDeg, closeTo(-45.0, 0.01));
    expect(hung.pitchDeg,
        closeTo(math.atan2(1.2 - 4.2, 3.0) * 180.0 / math.pi, 1e-9));
  });

  test('스피커를 옮기면 여러 밴드의 값이 실제로 달라진다', () {
    String sig(List<AutoEqBand> b) => b.join(';');
    expect(sig(bandsFor(x: 5.0, y: 3.0, z: 2.0)),
        isNot(sig(bandsFor(x: 0.4, y: 8.0, z: 0.5))));

    // 일반적인 현장 배치에서 충분한 수의 밴드가 실제로 동작해야 한다.
    final placed = bandsFor(x: 0.4, y: 9.0, z: 0.4);
    expect(placed.where((b) => b.enabled).length, greaterThanOrEqualTo(5),
        reason: '8밴드를 두고도 너무 적은 밴드만 쓴다');
  });

  // ── 서브우퍼 / 베이스 매니지먼트 ────────────────────────────────────
  // 서브우퍼는 저역 전용 스피커라, 풀레인지 스피커용 자동 보정을 그대로 받으면 안 된다.
  // 예전에는 서브로 지정한 스피커에도 일반 로우컷이 걸렸다. 벽 가까이 둔 서브는 경계면
  // 규칙 때문에 115Hz·24dB/oct 로우컷을 받아, 서브 출력(120Hz 이하)이 통째로 잘렸다
  // (실기 보고: "Set as LFE를 켜니 저음이 아예 안 들린다").

  test('서브우퍼는 일반 로우컷 대신 초저역 보호(서브소닉)만 받는다', () {
    // 저역 한계를 150Hz로 잡은 스피커: 일반 스피커로 쓰면 150Hz 로우컷이 걸린다.
    final asMain = bandsFor(y: 0.2, lowCut: 150.0);
    expect(asMain[kAutoEqSlotLowCut].freq, closeTo(150.0, 0.01));

    final asSub = bandsFor(y: 0.2, lowCut: 150.0, isSubwoofer: true);
    final lowCut = asSub[kAutoEqSlotLowCut];
    expect(lowCut.enabled, isTrue, reason: '서브도 초저역 보호는 둔다');
    expect(lowCut.freq, lessThanOrEqualTo(25.0),
        reason: '서브의 로우컷이 ${lowCut.freq}Hz다 — 서브 대역을 자르면 안 된다');
    expect(lowCut.slopeDbPerOct, 12);
  });

  test('서브우퍼에는 풀레인지용 보정(경계면 셸프·반사·공기 흡음·오프액시스)을 걸지 않는다', () {
    final sub = bandsFor(x: 0.3, y: 0.2, z: 0.3, yaw: 180.0, isSubwoofer: true);
    for (final slot in [
      kAutoEqSlotBoundary,
      kAutoEqSlotFloorBounce,
      kAutoEqSlotCeilingBounce,
      kAutoEqSlotWallBounce,
      kAutoEqSlotAirAbsorption,
      kAutoEqSlotOffAxis,
    ]) {
      expect(sub[slot].enabled, isFalse, reason: '서브에 슬롯 $slot 보정이 걸렸다: ${sub[slot]}');
    }
  });

  test('서브우퍼는 서브 대역(120Hz 이하)의 룸 모드만 보정한다', () {
    // 방 한가운데 바닥 가까이: 청취 지점에서 가로·세로 2차 모드(34Hz)가 가장 세다.
    final sub = bandsFor(x: 5.0, y: 5.0, z: 0.2, isSubwoofer: true);
    final mode = sub[kAutoEqSlotRoomMode];
    expect(mode.enabled, isTrue);
    expect(mode.freq, lessThanOrEqualTo(120.0));
  });

  test('베이스 매니지먼트 중인 메인 스피커는 EQ 로우컷을 크로스오버에 맡긴다', () {
    // 크로스오버가 이미 메인의 저역을 잘라 서브로 보낸다. EQ 로우컷(벽 근처면 115Hz)까지
    // 겹치면 두 번 잘려, 크로스오버(80Hz)와 로우컷 사이 대역이 메인에도 서브에도 없게 된다.
    final managed = bandsFor(y: 0.2, bassManagedCrossover: 80.0);
    expect(managed[kAutoEqSlotLowCut].enabled, isFalse);
  });

  test('스피커 한계가 크로스오버보다 높으면 보호용 로우컷은 남긴다', () {
    // 크로스오버(80Hz)보다 저역을 못 내는 작은 스피커(150Hz)는 보호가 필요하다.
    final managed = bandsFor(lowCut: 150.0, bassManagedCrossover: 80.0);
    expect(managed[kAutoEqSlotLowCut].enabled, isTrue);
    expect(managed[kAutoEqSlotLowCut].freq, greaterThanOrEqualTo(150.0));
  });

  // ── 헤드폰 미리듣기의 현장 물리 밴드 ────────────────────────────────
  // 자동 EQ는 현장 물리를 보정한다. 헤드폰에는 그 물리가 없어서 보정만 남아 음색이 틀어졌다.
  // 같은 모델로 물리를 만들어 헤드폰 경로에만 건다(Rust binaural.rs).

  test('물리 밴드는 8개이고 슬롯 역할이 고정된다', () {
    for (final p in [physicsFor(), physicsFor(x: 0.2, y: 9.5, z: 2.8, yaw: 170.0)]) {
      expect(p.length, kFieldPhysicsBandCount);
      expect(p.map((b) => b.slot).toList(), [0, 1, 2, 3, 4, 5, 6, 7]);
      expect(p[kPhysicsSlotBoundary].type, EqType.lowShelf);
      for (int k = 1; k <= 6; k++) {
        expect(p[k].type, EqType.bell);
      }
      expect(p[kPhysicsSlotDirectivity].type, EqType.highShelf);
    }
  });

  test('경계면 저음 증가는 자동 EQ 경계면 보정과 정확히 상쇄된다', () {
    final comp = slot(bandsFor(x: 0.3, y: 5.0, z: 1.5), kAutoEqSlotBoundary);
    final phys = physicsFor(x: 0.3, y: 5.0, z: 1.5)[kPhysicsSlotBoundary];
    expect(comp.enabled && phys.enabled, isTrue);
    expect(phys.gain, closeTo(-comp.gain, 1e-9));
    expect(phys.freq, comp.freq);
    expect(phys.q, comp.q);
  });

  test('룸 모드: 헤드폰에는 청취 지점에서 솟는 모드가 있고, 자동 EQ는 가장 센 것을 상쇄한다', () {
    final comp = slot(bandsFor(x: 5.0, y: 5.0, z: 0.2), kAutoEqSlotRoomMode);
    final phys = physicsFor(x: 5.0, y: 5.0, z: 0.2);
    final first = phys[kPhysicsSlotRoomModes];
    expect(first.enabled, isTrue);
    expect(first.gain, closeTo(-comp.gain, 1e-9));
    expect(first.freq, comp.freq);
    expect(first.q, comp.q);
    // 바닥 가까이라 높이축 2·3차 모드(114, 171Hz)도 청취 지점에서 솟는다(보정하지 않는 모드).
    final others = [phys[kPhysicsSlotRoomModes + 1], phys[kPhysicsSlotRoomModes + 2]];
    expect(others.where((b) => b.enabled).length, 2);
    for (final b in others) {
      expect(b.gain, greaterThan(0.0));
    }
  });

  test('근접면 반사: 헤드폰 초기반사가 빼는 가까운 면만 물리 밴드로 채운다', () {
    // 천장 5m 방, 천장에서 0.25m 아래 매단 스피커: 천장 반사는 경로차가 짧아 헤드폰
    // 초기반사 시뮬레이션이 뺀다 -> 물리 밴드로 그 첫 피크를 넣는다(자동 EQ는 절반을 깎는다).
    List<AutoEqBand> phys(double z) => computeFieldPhysicsBands(
          speakerX: 5.0, speakerY: 2.0, speakerZ: z,
          listenerX: 5.0, listenerY: 5.0, listenerZ: 1.2,
          roomWidth: 10.0, roomDepth: 10.0, ceilingHeight: 5.0,
          yawDeg: 0.0, autoAim: true, dispersionAngleDeg: 90.0,
          dispersionAngleVDeg: 90.0, pitchTiltDeg: 0.0, speedOfSound: kC,
        );
    List<AutoEqBand> comp(double z) => computeAutoEqBands(
          speakerX: 5.0, speakerY: 2.0, speakerZ: z,
          listenerX: 5.0, listenerY: 5.0, listenerZ: 1.2,
          roomWidth: 10.0, roomDepth: 10.0, ceilingHeight: 5.0,
          yawDeg: 0.0, autoAim: true, dispersionAngleDeg: 90.0,
          dispersionAngleVDeg: 90.0, pitchTiltDeg: 0.0,
          lowCutHz: 45.0, speedOfSound: kC,
        );
    final ceilingPhys = phys(4.75)[kPhysicsSlotNearReflections + 1];
    final ceilingComp = slot(comp(4.75), kAutoEqSlotCeilingBounce);
    expect(ceilingPhys.enabled, isTrue);
    expect(ceilingComp.enabled, isTrue);
    expect(ceilingPhys.freq, ceilingComp.freq);
    expect(ceilingPhys.gain, greaterThan(-ceilingComp.gain),
        reason: '물리 피크가 보정(절반)보다 커야 현장처럼 절반이 남는다');

    // 바닥은 멀다(경로차 1.5m 초과) -> 헤드폰 초기반사가 이미 만든다. 물리 밴드로 또 넣지 않는다.
    expect(phys(4.75)[kPhysicsSlotNearReflections].enabled, isFalse);
  });

  test('서브우퍼 물리 밴드는 경계면·룸 모드만 (반사 피크·지향성은 서브 대역 밖)', () {
    final p = physicsFor(x: 0.3, y: 0.3, z: 0.3, yaw: 180.0, isSubwoofer: true);
    expect(p[kPhysicsSlotBoundary].enabled, isTrue, reason: '벽·코너의 저역 증가는 서브에도 생긴다');
    for (int k = kPhysicsSlotNearReflections; k < kPhysicsSlotNearReflections + 3; k++) {
      expect(p[k].enabled, isFalse);
    }
    expect(p[kPhysicsSlotDirectivity].enabled, isFalse);
  });
}
