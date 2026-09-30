import 'dart:math' as math;
import 'package:atmos_mixer_pro/src/rust/common/config.dart';

/// 스피커 배치(위치·높이·각도)로부터 물리적으로 유도되는 EQ 자동 계산.
///
/// 이 파일은 순수 함수만 담는다. Riverpod / FFI 의존성이 없으므로
/// `test/position_eq_test.dart`에서 숫자를 그대로 검증할 수 있다.
///
/// ## 슬롯 정책
/// 8밴드 중 앞의 5개를 자동 계산이 소유하고, 뒤의 3개(5·6·7)는 언제나
/// 엔지니어 몫으로 비워둔다. 슬롯을 역할별로 **고정**하는 이유는, 예전처럼
/// "비어 있는 밴드를 찾아 끼워넣는" 방식이면 스피커를 조금 움직여 어떤 역할이
/// 껐다 켜질 때마다 밴드가 자리를 옮겨 다녀서 EQ 그래프가 요동치기 때문이다.
///
/// 자동 슬롯이라도 `bandAuto[i] == false`(엔지니어가 손댄 밴드)면 덮어쓰지
/// 않는다. 그게 "자동으로 잡아주고, 현장에서 엔지니어가 고쳐 쓴다"는 요구다.
const int kAutoEqSlotLowCut = 0;
const int kAutoEqSlotBoundary = 1;
const int kAutoEqSlotRoomMode = 2;
const int kAutoEqSlotFloorBounce = 3;
const int kAutoEqSlotCeilingBounce = 4;
const int kAutoEqSlotWallBounce = 5;
const int kAutoEqSlotAirAbsorption = 6;
const int kAutoEqSlotOffAxis = 7;

/// 자동 계산이 8밴드를 모두 쓴다. 엔지니어가 어떤 밴드든 손대면 그 밴드는
/// 자동에서 떨어져 나와(bandAuto=false) 더 이상 덮어쓰지 않는다.
const int kAutoEqSlotCount = 8;

/// 자동 계산 결과 밴드 하나. `slot`은 위 상수 중 하나다.
class AutoEqBand {
  final int slot;
  final bool enabled;
  final EqType type;
  final double freq;
  final double gain;
  final double q;

  /// 컷 필터의 기울기(dB/oct). 벨·쉘프에서는 쓰이지 않는다.
  ///
  /// `ChannelTuningState.bandSlopes`에 대응한다. 이 필드가 생기기 전에는
  /// 자동 계산이 슬로프를 아예 안 건드려서 로우컷이 늘 12dB/oct 기본값으로
  /// 고정돼 있었다.
  final int slopeDbPerOct;

  const AutoEqBand({
    required this.slot,
    required this.enabled,
    required this.type,
    required this.freq,
    required this.gain,
    required this.q,
    this.slopeDbPerOct = 12,
  });

  @override
  String toString() =>
      'AutoEqBand(slot: $slot, on: $enabled, ${type.name}, '
      '${freq.toStringAsFixed(1)}Hz, ${gain.toStringAsFixed(2)}dB, '
      'Q${q.toStringAsFixed(2)}, ${slopeDbPerOct}dB/oct)';
}

/// 스피커 한 대의 배치로부터 자동 EQ 5밴드를 계산한다.
///
/// 항상 [kAutoEqSlotCount]개를 슬롯 순서대로 돌려준다. 해당 보정이 필요 없는
/// 상황이면 `enabled: false`, `gain: 0`인 밴드가 들어 있다(밴드를 끄는 것도
/// 자동 계산의 결과다).
///
/// 좌표계는 스피커 레이아웃과 동일한 미터 단위: x = 가로, y = 깊이,
/// z = 바닥으로부터의 높이.
List<AutoEqBand> computeAutoEqBands({
  required double speakerX,
  required double speakerY,
  required double speakerZ,
  required double listenerX,
  required double listenerY,
  required double listenerZ,
  required double roomWidth,
  required double roomDepth,
  required double ceilingHeight,
  required double yawDeg,
  /// yaw를 엔지니어가 지정하지 않았으면 청취 지점을 향해 조준한 것으로 본다.
  /// 자세한 이유는 `SpeakerNode.autoAim` 주석 참고.
  required bool autoAim,
  required double dispersionAngleDeg,
  /// 수직 분산각(도). 수평과 달리 대개 더 좁다.
  required double dispersionAngleVDeg,
  /// 상하 조준각(도). **음수 = 아래**, 양수 = 위(`SpeakerNode.pitchTilt` 규약).
  required double pitchTiltDeg,
  required double lowCutHz,
  required double speedOfSound,
  /// 방의 브로드밴드 흡음계수(0~1). 반사 보정량과 룸 모드 Q를 정한다.
  double absorptionCoeff = 0.3,
  /// 이 스피커가 베이스 매니지먼트의 서브우퍼(Set as LFE Subwoofer)인가.
  /// 서브는 저역 전용이라 풀레인지용 보정 대신 초저역 보호와 서브 대역 룸 모드만 받는다.
  bool isSubwoofer = false,
  /// 베이스 매니지먼트로 저역을 서브에 넘기는 메인 스피커라면 그 크로스오버 주파수(Hz).
  /// 크로스오버가 메인의 저역을 이미 잘라 서브로 보내므로 EQ 로우컷을 겹치지 않는다.
  double? bassManagedCrossoverHz,
}) {
  final double dx = speakerX - listenerX;
  final double dy = speakerY - listenerY;
  final double dz = speakerZ - listenerZ;
  final double distance = math.sqrt(dx * dx + dy * dy + dz * dz);
  final double c = speedOfSound <= 0 ? 343.0 : speedOfSound;

  // ── 슬롯 0: 스피커 보호용 로우컷 ──────────────────────────────
  // 스피커 저역 한계(lowCutHz, 인스펙터에서 스피커마다 설정)를 그대로 12dB/oct로 건다.
  //
  // 예전에는 경계면에 가까우면 차단 주파수를 최대 1.5배 올리고 기울기도 24dB/oct까지
  // 세웠다. 경계면이 만드는 저역 부풂은 슬롯 1 로우셸프가 이미 보정하므로 이중
  // 보정이었고, 벽에서 0.2m인 스피커는 115Hz·24dB/oct가 되어 저역이 통째로
  // 빠졌다(실기 보고: "EQ에서 로우컷이 많이 된다").
  final lowCut = AutoEqBand(
    slot: kAutoEqSlotLowCut,
    enabled: lowCutHz > 20.0,
    type: EqType.lowCut,
    freq: _round2(lowCutHz.clamp(20.0, 400.0)),
    gain: 0.0, // 로우컷은 게인 파라미터를 쓰지 않는다
    q: 0.707,
    slopeDbPerOct: 12,
  );

  // ── 슬롯 1: 경계면 저음 부스트 보정(SBIR) ──────────────────────
  // 경계면이 가까우면 그 면의 반사가 저역에서 동상으로 더해져 부밍이 생긴다.
  // 부밍이 끝나고 첫 상쇄가 시작되는 경계가 사분파장 지점 f = c / (4d)이므로,
  // 그 아래를 로우쉘프로 눌러준다.
  //
  // 예전에는 주파수를 150Hz로 **고정**해서, 스피커를 어디로 옮겨도 이 밴드의
  // 주파수가 그대로였다.
  //
  // 보정량은 헤드폰 미리듣기의 물리 밴드(computeFieldPhysicsBands)와 같은 모델이다.
  final loading = _boundaryLoading(
    speakerX: speakerX,
    speakerY: speakerY,
    speakerZ: speakerZ,
    roomWidth: roomWidth,
    roomDepth: roomDepth,
    ceilingHeight: ceilingHeight,
    speedOfSound: c,
  );
  final boundary = AutoEqBand(
    slot: kAutoEqSlotBoundary,
    enabled: loading.gainDb > 0.2,
    type: EqType.lowShelf,
    freq: _round2(loading.freq),
    gain: _round2(-loading.gainDb),
    q: 0.707,
  );

  // ── 슬롯 2: 룸 모드(축 공진) ────────────────────────────────────
  // 청취 지점에서 가장 크게 솟는 축 모드 하나를 벨로 눌러준다. 모델은 roomModePeaks 참고.
  //
  // 예전에는 음원 쪽 여기(|cos(pi*x/L)|)만 보고 1차 모드를 골랐다. 그런데 청취 지점인 방
  // 한가운데는 홀수 차수 모드의 마디라 1차 모드가 거의 안 들리고, 오히려 2차 모드의 배다.
  // 그래서 실제로는 없는 공진을 깎아 그 주파수에 구멍을 냈다(예: 5m 방에서 34Hz).
  final double absorption = absorptionCoeff.clamp(0.0, 1.0);
  final modes = roomModePeaks(
    speakerX: speakerX,
    speakerY: speakerY,
    speakerZ: speakerZ,
    listenerX: listenerX,
    listenerY: listenerY,
    listenerZ: listenerZ,
    roomWidth: roomWidth,
    roomDepth: roomDepth,
    ceilingHeight: ceilingHeight,
    speedOfSound: c,
    absorptionCoeff: absorption,
  );
  final RoomModePeak? strongestMode = modes.isEmpty ? null : modes.first;
  final roomMode = AutoEqBand(
    slot: kAutoEqSlotRoomMode,
    enabled: strongestMode != null && strongestMode.gainDb > 0.2,
    type: EqType.bell,
    freq: strongestMode != null ? _round2(strongestMode.freq) : 60.0,
    gain: strongestMode != null ? _round2(-strongestMode.gainDb) : 0.0,
    q: _round2(strongestMode?.q ?? _roomModeQ(absorption)),
  );

  // ── 슬롯 2: 바닥 반사 콤필터 첫 딥 보정 ──────────────────────────
  // 바닥(z=0)에 대한 이미지 소스와 직접음의 경로차 Δ가 만드는 첫 상쇄
  // 주파수 f = c / (2Δ)를 벨로 눌러준다. 스피커 높이를 바꾸면 바로 움직이는
  // 값이라, 배치 변경이 EQ에 반영되는지 귀로 확인하기도 쉽다.
  final reflections = _surfaceReflections(
    speakerX: speakerX,
    speakerY: speakerY,
    speakerZ: speakerZ,
    listenerX: listenerX,
    listenerY: listenerY,
    listenerZ: listenerZ,
    roomWidth: roomWidth,
    roomDepth: roomDepth,
    ceilingHeight: ceilingHeight,
    absorption: absorption,
  );
  final floorBounce = _reflectionBand(reflections[0], distance, c);
  final ceilingBounce = _reflectionBand(reflections[1], distance, c);
  final wallBounce = _reflectionBand(reflections[2], distance, c);

  // ── 슬롯 3: 공기 흡음 고역 롤오프 보상 ──────────────────────────
  // 5m를 넘는 거리부터 1m당 +0.5dB, 최대 +6dB까지 되살린다.
  double airGain = 0.0;
  if (distance > 5.0) {
    airGain = ((distance - 5.0) * 0.5).clamp(0.0, 6.0);
  }
  final airAbsorption = AutoEqBand(
    slot: kAutoEqSlotAirAbsorption,
    enabled: airGain > 0.05,
    type: EqType.highShelf,
    freq: 10000.0,
    gain: _round2(airGain),
    q: 0.707,
  );

  // ── 슬롯 7: 오프액시스 — 자동 EQ로는 걸지 않는다 ─────────────────
  // 청취 지점이 스피커 커버리지 콘 밖이면 고역이 먼저 빠진다. 그건 스피커가 실제로 만드는
  // 현상이라 현장에서는 이미 일어난다. 예전에는 그걸 EQ 컷으로 한 번 더 걸어서 현장에서
  // 고역이 두 번 깎였다(공기 흡음과 같은 문제). 이제 헤드폰 미리듣기의 물리 밴드(지향성,
  // computeFieldPhysicsBands)로만 흉내 낸다. 슬롯 자리는 그대로 두고 꺼 둔다.
  final offAxis = AutoEqBand(
    slot: kAutoEqSlotOffAxis,
    enabled: false,
    type: EqType.highShelf,
    freq: 4000.0,
    gain: 0.0,
    q: 0.707,
  );

  final bands = <AutoEqBand>[
    lowCut,
    boundary,
    roomMode,
    floorBounce,
    ceilingBounce,
    wallBounce,
    airAbsorption,
    offAxis,
  ];
  if (isSubwoofer) return _asSubwoofer(bands);
  // 크로스오버가 메인의 저역을 이미 자른다. EQ 로우컷(벽 근처면 115Hz·24dB/oct)까지
  // 겹치면 두 번 잘려서, 크로스오버와 로우컷 사이 대역이 메인에도 서브에도 없게 된다.
  // 스피커 자체 한계가 크로스오버보다 높을 때만 보호용으로 남긴다.
  if (bassManagedCrossoverHz != null && lowCutHz <= bassManagedCrossoverHz) {
    bands[kAutoEqSlotLowCut] = _disabled(bands[kAutoEqSlotLowCut]);
  }
  return bands;
}

/// 스피커 지향성: 청취 지점이 커버리지 콘 밖일 때 고역(4kHz 셸프) 감쇠량(dB, 0 이하).
///
/// 스피커가 실제로 만드는 현상이라 자동 EQ로는 걸지 않고 헤드폰 물리 밴드로만 흉내 낸다.
/// 조준하지 않은 스피커(autoAim)는 청취 지점을 향한 것으로 본다.
double _offAxisLossDb({
  required double speakerX,
  required double speakerY,
  required double speakerZ,
  required double listenerX,
  required double listenerY,
  required double listenerZ,
  required double yawDeg,
  required bool autoAim,
  required double dispersionAngleDeg,
  required double dispersionAngleVDeg,
  required double pitchTiltDeg,
}) {
  final double dx = speakerX - listenerX;
  final double dy = speakerY - listenerY;
  final double horiz = math.sqrt(dx * dx + dy * dy);
  // 청취 지점이 스피커 커버리지 콘 밖이면 고역이 먼저 빠진다.
  // 수평(yaw / dispersionAngle)과 수직(pitch / dispersionAngleV)을 각각
  // 계산해서 더한다. 두 축 모두 벗어났으면 실제로도 더 어두워진다.
  //
  // 좌표 규약:
  // - yaw 0°의 정면은 레이아웃 +y다. 3D 씬이 로컬 +Z를 정면으로 쓰고
  //   world Z <- 레이아웃 y로 매핑한다(studio_engine.html의 laserPoints,
  //   `group.position.set(posX, posY, posZ)`).
  // - pitchTilt는 **음수가 아래**다('Auto-Aim to Listener' 버튼과 3D 씬의
  //   `rotation.x = -pitchRad`가 쓰는 규약).
  double offAxisGain = 0.0;
  double thetaH = 0.0;
  double thetaV = 0.0;
  if (!autoAim) {
    // 수평 성분
    if (dispersionAngleDeg > 0.0 && horiz > 1e-6) {
      final double yawRad = yawDeg * math.pi / 180.0;
      final double fx = math.sin(yawRad);
      final double fy = math.cos(yawRad);
      // 스피커 -> 청취자 방향(위의 dx/dy는 청취자 -> 스피커라 부호를 뒤집는다)
      final double tx = -dx / horiz;
      final double ty = -dy / horiz;
      final double cosTheta = (fx * tx + fy * ty).clamp(-1.0, 1.0);
      thetaH = math.acos(cosTheta) * 180.0 / math.pi;
    }
    // 수직 성분: 조준선의 상하각 vs 스피커에서 본 청취자의 상하각 차이.
    // 둘 다 "양수 = 위" 규약으로 맞춰서 뺀다.
    if (dispersionAngleVDeg > 0.0) {
      final double listenerElevation =
          math.atan2(listenerZ - speakerZ, math.max(horiz, 1e-6)) *
              180.0 /
              math.pi;
      thetaV = (pitchTiltDeg - listenerElevation).abs();
    }

    // 콘 가장자리에서 1도 벗어날 때마다 0.2dB. 90도 콘 기준으로 정면에서
    // 90도 틀어지면 약 -9dB, 완전히 등지면 상한 -18dB가 된다. 실제 스피커의
    // 4kHz 폴라 응답(후면 15~25dB 감쇠)과 같은 자릿수다.
    //
    // 예전 Rust 구현은 0.5dB/도에 상한 24dB였는데, 콘에서 48도만 벗어나도
    // 상한에 닿아 사실상 고역을 지워버렸다. 그 코드는 실제 오디오 경로까지
    // 도달한 적이 없어 들어본 사람이 없었다.
    double excess = 0.0;
    final double halfH = dispersionAngleDeg / 2.0;
    final double halfV = dispersionAngleVDeg / 2.0;
    if (dispersionAngleDeg > 0.0 && thetaH > halfH) excess += thetaH - halfH;
    if (dispersionAngleVDeg > 0.0 && thetaV > halfV) excess += thetaV - halfV;
    if (excess > 0.0) {
      offAxisGain = -math.min(excess * 0.2, 18.0);
    }
  }
  return offAxisGain;
}

/// 서브우퍼 초저역 보호(서브소닉) 주파수. 서브 대역은 건드리지 않는다.
const double kSubsonicHz = 20.0;

/// 서브우퍼 대역 상한(Hz). LFE 규격 대역과 같다.
const double kSubwooferBandTopHz = 120.0;

/// 자동 계산이 "이 보정은 필요 없다"고 판단한 밴드(꺼짐, 게인 0).
AutoEqBand _disabled(AutoEqBand b) => AutoEqBand(
      slot: b.slot,
      enabled: false,
      type: b.type,
      freq: b.freq,
      gain: 0.0,
      q: b.q,
      slopeDbPerOct: b.slopeDbPerOct,
    );

/// 서브우퍼용 자동 EQ.
///
/// - 로우컷: 풀레인지용(스펙 80Hz, 벽 근처 최대 120Hz·24dB/oct)이면 서브 출력이
///   통째로 잘린다. 대신 20Hz 서브소닉 보호만 둔다.
/// - 룸 모드: 서브 대역(120Hz 이하)이면 그대로 보정한다 — 서브 EQ의 본래 목적이다.
/// - 경계면 셸프: 서브를 벽·코너에 두는 건 저역 효율을 얻으려는 의도라 깎지 않는다.
/// - 반사 보정·공기 흡음·오프액시스: 풀레인지용 중·고역 보정이라 서브에는 걸지 않는다.
List<AutoEqBand> _asSubwoofer(List<AutoEqBand> bands) => [
      for (final b in bands)
        switch (b.slot) {
          kAutoEqSlotLowCut => AutoEqBand(
              slot: b.slot,
              enabled: true,
              type: EqType.lowCut,
              freq: kSubsonicHz,
              gain: 0.0,
              q: 0.707,
              slopeDbPerOct: 12,
            ),
          kAutoEqSlotRoomMode => b.freq <= kSubwooferBandTopHz ? b : _disabled(b),
          _ => _disabled(b),
        },
    ];

/// 반사면 하나(이미지 소스)의 기하: 수평 거리, 거울상까지의 수직 거리, 면의 흡음, 보정 상한.
typedef _SurfaceReflection = ({
  int slot,
  double horiz,
  double mirroredDz,
  double absorption,
  double maxCutDb,
});

/// 자동 EQ가 보정하는 세 반사면(바닥·천장·가까운 벽)의 이미지 소스 기하.
/// 보정(자동 EQ)과 헤드폰 물리 밴드가 같은 기하를 쓰도록 한 곳에서 만든다.
List<_SurfaceReflection> _surfaceReflections({
  required double speakerX,
  required double speakerY,
  required double speakerZ,
  required double listenerX,
  required double listenerY,
  required double listenerZ,
  required double roomWidth,
  required double roomDepth,
  required double ceilingHeight,
  required double absorption,
}) {
  final double dx = speakerX - listenerX;
  final double dy = speakerY - listenerY;
  final double dz = speakerZ - listenerZ;
  final double horiz = math.sqrt(dx * dx + dy * dy);
  final double distWallX = math.min(speakerX, roomWidth - speakerX);
  final double distWallY = math.min(speakerY, roomDepth - speakerY);
  final double nearestWall = math.min(distWallX, distWallY);
  final double listenerWall = distWallX <= distWallY
      ? math.min(listenerX, roomWidth - listenerX)
      : math.min(listenerY, roomDepth - listenerY);
  final double wallParallel = distWallX <= distWallY
      ? (speakerY - listenerY).abs()
      : (speakerX - listenerX).abs();
  return [
    (
      slot: kAutoEqSlotFloorBounce,
      horiz: horiz,
      mirroredDz: speakerZ + listenerZ, // 바닥(z=0) 거울상
      absorption: absorption,
      maxCutDb: 4.0,
    ),
    (
      slot: kAutoEqSlotCeilingBounce,
      horiz: horiz,
      mirroredDz: (ceilingHeight - speakerZ) + (ceilingHeight - listenerZ),
      // 천장은 보통 흡음이 덜 돼 있다.
      absorption: (absorption * 0.7).clamp(0.0, 1.0),
      maxCutDb: 4.0,
    ),
    (
      slot: kAutoEqSlotWallBounce,
      horiz: math.sqrt(wallParallel * wallParallel + dz * dz),
      mirroredDz: nearestWall + listenerWall,
      absorption: absorption,
      maxCutDb: 3.5,
    ),
  ];
}

/// 반사면 하나가 만드는 콤필터의 **첫 보강 피크**(없으면 null).
///
/// 이미지 소스 경로차 d = sqrt(horiz^2 + mirroredDz^2) - 직접거리에 대해
/// 보강(peak)은 f = c / d, 상쇄(null)는 f = c / (2d)에서 일어난다.
/// 반사 진폭 r = sqrt(1 - 흡음) x (직접거리 / 반사거리)이고 피크 상승폭은 20*log10(1 + r)이다.
/// taper: 유효 구간(60~800Hz) 밖에서는 서서히 빼서, 스피커를 옮기는 도중 밴드가 뚝 꺼지며
/// 딸깍거리지 않게 한다.
({double freq, double peakDb, double q, double taper, double pathDiff})? _reflectionPeak(
  _SurfaceReflection s,
  double directDistance,
  double speedOfSound,
) {
  final double reflected = math.sqrt(s.horiz * s.horiz + s.mirroredDz * s.mirroredDz);
  final double pathDiff = reflected - directDistance;
  if (pathDiff <= 0.01 || reflected <= 0.0) return null;
  final double freq = speedOfSound / pathDiff;
  final double r = (math.sqrt(math.max(0.0, 1.0 - s.absorption)) *
          (directDistance / reflected))
      .clamp(0.0, 1.0);
  final double peakDb = 20.0 * math.log(1.0 + r) / math.ln10;
  const double fullLow = 60.0, fullHigh = 800.0;
  const double fadeLow = 40.0, fadeHigh = 1500.0;
  double taper;
  if (freq >= fullLow && freq <= fullHigh) {
    taper = 1.0;
  } else if (freq > fadeLow && freq < fullLow) {
    taper = (freq - fadeLow) / (fullLow - fadeLow);
  } else if (freq > fullHigh && freq < fadeHigh) {
    taper = (fadeHigh - freq) / (fadeHigh - fullHigh);
  } else {
    taper = 0.0;
  }
  // 반사가 셀수록 피크가 좁고 뚜렷하다.
  final double q = (1.5 + 2.0 * r).clamp(1.5, 5.0);
  return (freq: freq, peakDb: peakDb, q: q, taper: taper, pathDiff: pathDiff);
}

/// 반사면 하나가 만드는 콤필터의 **첫 보강 피크**를 벨 한 밴드로 누른다.
///
/// 예전 구현은 **상쇄 지점**을 깎았다. 이미 반사로 소리가 빠진 골짜기를 EQ로
/// 더 깎는 것이라 음향적으로 역효과였다(딥은 EQ로 메울 수 없고, 깎으면 더
/// 깊어질 뿐이다). 보정할 수 있는 건 솟아오른 피크 쪽이다.
///
/// 콤필터는 EQ로 완전히 지울 수 없으므로 피크 상승폭의 절반만 보정하고 상한을 둔다.
AutoEqBand _reflectionBand(
  _SurfaceReflection s,
  double directDistance,
  double speedOfSound,
) {
  final peak = _reflectionPeak(s, directDistance, speedOfSound);
  final double gain =
      peak == null ? 0.0 : -math.min(peak.peakDb * 0.5, s.maxCutDb) * peak.taper;
  final bool on = gain < -0.05;
  return AutoEqBand(
    slot: s.slot,
    enabled: on,
    type: EqType.bell,
    freq: on ? _round2(peak!.freq) : 200.0,
    gain: _round2(gain),
    q: _round2(peak?.q ?? 3.0),
  );
}

/// 경계면 저음 증가: 벽·바닥·천장이 가까우면 그 면의 반사가 저역에서 동상으로 더해져
/// 부밍이 생긴다. 부밍이 끝나고 첫 상쇄가 시작되는 경계가 사분파장 지점 f = c / (4d)다.
/// (증가량 dB ≥ 0, 셸프 주파수 Hz). 자동 EQ는 이만큼 깎고, 헤드폰 물리 밴드는 이만큼 올린다.
({double gainDb, double freq}) _boundaryLoading({
  required double speakerX,
  required double speakerY,
  required double speakerZ,
  required double roomWidth,
  required double roomDepth,
  required double ceilingHeight,
  required double speedOfSound,
}) {
  final distances = <double>[
    speakerX,
    roomWidth - speakerX,
    speakerY,
    roomDepth - speakerY,
    speakerZ,
    ceilingHeight - speakerZ,
  ];
  double loss = 0.0;
  for (final d in distances) {
    loss += -3.0 * math.exp(-math.max(d, 0.0) / 0.7);
  }
  final double tightest = distances.reduce(math.min);
  return (
    gainDb: -loss.clamp(-9.0, 0.0),
    freq: (speedOfSound / (4.0 * math.max(tightest, 0.05))).clamp(40.0, 300.0),
  );
}

/// 청취 지점에서 솟는 축 모드(방의 한 축을 오가는 정재파) 하나.
class RoomModePeak {
  final double freq;
  /// 청취 지점에서 솟는 크기(dB, 양수).
  final double gainDb;
  final double q;
  /// 음원 여기 x 청취 지점 크기(0~1).
  final double coupling;

  const RoomModePeak({
    required this.freq,
    required this.gainDb,
    required this.q,
    required this.coupling,
  });
}

double _roomModeQ(double absorption) =>
    (2.0 + 6.0 * (1.0 - absorption)).clamp(2.0, 8.0);

/// 청취 지점에서 솟는 축 모드들(센 순서).
///
/// 축 모드 f = n·c / (2L) (n = 1, 2, 3)의 음압은 벽에서 가장 크고 cos(nπx/L) 모양이다.
/// 음원 쪽 여기와 청취 지점의 크기를 곱한 |cos(nπ·음원/L)·cos(nπ·청취/L)|가 그 모드가
/// 청취 지점에서 들리는 정도다. 방 한가운데는 홀수 차수의 마디(0)이고 짝수 차수의 배(1)다.
/// 크기는 (1 + 4·결합)·(1 − 흡음)dB, Q는 흡음이 적을수록 좁다. 20~250Hz, 결합 0.45 초과만.
/// 같은 주파수(2% 이내)의 모드는 겹쳐서 더 세게 울리므로 합친다(정사각형 방, 최대 9dB).
List<RoomModePeak> roomModePeaks({
  required double speakerX,
  required double speakerY,
  required double speakerZ,
  required double listenerX,
  required double listenerY,
  required double listenerZ,
  required double roomWidth,
  required double roomDepth,
  required double ceilingHeight,
  required double speedOfSound,
  required double absorptionCoeff,
}) {
  final double c = speedOfSound <= 0 ? 343.0 : speedOfSound;
  final double absorption = absorptionCoeff.clamp(0.0, 1.0);
  final raw = <RoomModePeak>[];
  for (final axis in <List<double>>[
    [roomWidth, speakerX, listenerX],
    [roomDepth, speakerY, listenerY],
    [ceilingHeight, speakerZ, listenerZ],
  ]) {
    final double length = axis[0];
    if (length <= 0.1) continue;
    for (int n = 1; n <= 3; n++) {
      final double f = n * c / (2.0 * length);
      if (f < 20.0 || f > 250.0) continue;
      final double coupling = (math.cos(n * math.pi * (axis[1] / length).clamp(0.0, 1.0)) *
              math.cos(n * math.pi * (axis[2] / length).clamp(0.0, 1.0)))
          .abs();
      if (coupling <= 0.45) continue;
      raw.add(RoomModePeak(
        freq: f,
        gainDb: (1.0 + 4.0 * coupling) * (1.0 - absorption),
        q: _roomModeQ(absorption),
        coupling: coupling,
      ));
    }
  }
  raw.sort((a, b) => a.freq.compareTo(b.freq));
  final merged = <RoomModePeak>[];
  for (final m in raw) {
    if (merged.isNotEmpty && (m.freq - merged.last.freq).abs() <= merged.last.freq * 0.02) {
      final last = merged.removeLast();
      merged.add(RoomModePeak(
        freq: last.freq,
        gainDb: math.min(last.gainDb + m.gainDb, 9.0),
        q: last.q,
        coupling: math.max(last.coupling, m.coupling),
      ));
    } else {
      merged.add(m);
    }
  }
  merged.sort((a, b) => b.gainDb.compareTo(a.gainDb));
  return merged;
}

// ── 헤드폰 미리듣기의 현장 물리 밴드 ─────────────────────────────────
// 자동 EQ는 현장 물리를 **보정**한다. 현장에서는 실제 물리가 그 보정을 상쇄하지만, 헤드폰
// 미리듣기(바이노럴)에는 그 물리가 없어서 보정만 남아 음색이 틀어졌다(예: 경계면 보정만 남아
// 저역이 얇음). 그래서 같은 모델로 물리를 만들어 헤드폰 경로에만 건다(Rust binaural.rs).

/// 채널당 물리 밴드 수(슬롯 고정). Rust binaural::MAX_SIM_BANDS와 같아야 한다.
const int kFieldPhysicsBandCount = 8;
const int kPhysicsSlotBoundary = 0; // 경계면 저음 증가(로우셸프 +)
const int kPhysicsSlotRoomModes = 1; // 룸 모드 피크 1~3(벨 +)
const int kPhysicsRoomModeCount = 3;
const int kPhysicsSlotNearReflections = 4; // 근접면 반사 첫 피크 4~6: 바닥·천장·가까운 벽(벨 +)
const int kPhysicsSlotDirectivity = 7; // 스피커 지향성(분산각 밖 고역 감쇠, 하이셸프 −)

/// 헤드폰 초기반사 시뮬레이션은 스피커에 붙은 면의 반사를 뺀다(경로차 0.5m 이하 0, 1.5m까지
/// 서서히 — Rust acoustic.rs NEAR_SURFACE_FADE_START_M/END_M과 같은 값). 뺀 만큼을 물리
/// 밴드로 채운다(각 반사면을 한 번씩만 센다).
double _nearSurfaceWeight(double pathDiff) =>
    1.0 - ((pathDiff - 0.5) / (1.5 - 0.5)).clamp(0.0, 1.0);

/// 스피커 한 대의 현장 물리 밴드([kFieldPhysicsBandCount]개, 슬롯 순서 고정).
///
/// 헤드폰 미리듣기 전용이다. 자동 EQ([computeAutoEqBands])와 **같은 모델**이라, 보정을 자동에
/// 맡기면 헤드폰에서도 현장처럼 상쇄된다. 엔지니어가 EQ를 바꾸면 그 차이가 그대로 들린다.
/// - 경계면 저음 증가: 자동 EQ 경계면 셸프가 깎는 만큼 올린다.
/// - 룸 모드: 청취 지점에서 솟는 모드 최대 3개(자동 EQ는 가장 센 하나를 깎는다).
/// - 근접면 반사 첫 피크: 헤드폰 초기반사가 빼는 가까운 면만큼(자동 EQ는 절반을 깎는다).
/// - 지향성: 청취 지점이 커버리지 콘 밖일 때 고역 감쇠(자동 EQ로는 걸지 않는다).
/// 서브우퍼는 경계면·룸 모드만 받는다(반사 피크·지향성은 서브 대역 밖이다).
List<AutoEqBand> computeFieldPhysicsBands({
  required double speakerX,
  required double speakerY,
  required double speakerZ,
  required double listenerX,
  required double listenerY,
  required double listenerZ,
  required double roomWidth,
  required double roomDepth,
  required double ceilingHeight,
  required double yawDeg,
  required bool autoAim,
  required double dispersionAngleDeg,
  required double dispersionAngleVDeg,
  required double pitchTiltDeg,
  required double speedOfSound,
  double absorptionCoeff = 0.3,
  bool isSubwoofer = false,
}) {
  final double dx = speakerX - listenerX;
  final double dy = speakerY - listenerY;
  final double dz = speakerZ - listenerZ;
  final double distance = math.sqrt(dx * dx + dy * dy + dz * dz);
  final double c = speedOfSound <= 0 ? 343.0 : speedOfSound;
  final double absorption = absorptionCoeff.clamp(0.0, 1.0);

  AutoEqBand bell(int slot, double freq, double gain, double q) => AutoEqBand(
        slot: slot,
        enabled: gain > 0.05,
        type: EqType.bell,
        freq: _round2(freq),
        gain: _round2(gain),
        q: _round2(q),
      );
  AutoEqBand bellOff(int slot) =>
      AutoEqBand(slot: slot, enabled: false, type: EqType.bell, freq: 200.0, gain: 0.0, q: 3.0);

  final bands = <AutoEqBand>[];
  final loading = _boundaryLoading(
    speakerX: speakerX,
    speakerY: speakerY,
    speakerZ: speakerZ,
    roomWidth: roomWidth,
    roomDepth: roomDepth,
    ceilingHeight: ceilingHeight,
    speedOfSound: c,
  );
  bands.add(AutoEqBand(
    slot: kPhysicsSlotBoundary,
    enabled: loading.gainDb > 0.2,
    type: EqType.lowShelf,
    freq: _round2(loading.freq),
    gain: _round2(loading.gainDb),
    q: 0.707,
  ));

  final modes = roomModePeaks(
    speakerX: speakerX,
    speakerY: speakerY,
    speakerZ: speakerZ,
    listenerX: listenerX,
    listenerY: listenerY,
    listenerZ: listenerZ,
    roomWidth: roomWidth,
    roomDepth: roomDepth,
    ceilingHeight: ceilingHeight,
    speedOfSound: c,
    absorptionCoeff: absorption,
  );
  for (int k = 0; k < kPhysicsRoomModeCount; k++) {
    final int slot = kPhysicsSlotRoomModes + k;
    bands.add(k < modes.length && modes[k].gainDb > 0.2
        ? bell(slot, modes[k].freq, modes[k].gainDb, modes[k].q)
        : bellOff(slot));
  }

  final reflections = _surfaceReflections(
    speakerX: speakerX,
    speakerY: speakerY,
    speakerZ: speakerZ,
    listenerX: listenerX,
    listenerY: listenerY,
    listenerZ: listenerZ,
    roomWidth: roomWidth,
    roomDepth: roomDepth,
    ceilingHeight: ceilingHeight,
    absorption: absorption,
  );
  for (int k = 0; k < reflections.length; k++) {
    final int slot = kPhysicsSlotNearReflections + k;
    final peak = isSubwoofer ? null : _reflectionPeak(reflections[k], distance, c);
    bands.add(peak == null
        ? bellOff(slot)
        : bell(slot, peak.freq, peak.peakDb * peak.taper * _nearSurfaceWeight(peak.pathDiff), peak.q));
  }

  final double loss = isSubwoofer
      ? 0.0
      : _offAxisLossDb(
          speakerX: speakerX,
          speakerY: speakerY,
          speakerZ: speakerZ,
          listenerX: listenerX,
          listenerY: listenerY,
          listenerZ: listenerZ,
          yawDeg: yawDeg,
          autoAim: autoAim,
          dispersionAngleDeg: dispersionAngleDeg,
          dispersionAngleVDeg: dispersionAngleVDeg,
          pitchTiltDeg: pitchTiltDeg,
        );
  bands.add(AutoEqBand(
    slot: kPhysicsSlotDirectivity,
    enabled: loss < -0.2,
    type: EqType.highShelf,
    freq: 4000.0,
    gain: _round2(loss),
    q: 0.707,
  ));
  return bands;
}

/// 소수 둘째 자리 반올림. 미세한 부동소수 흔들림이 그대로 백엔드 커맨드가
/// 되어 오디오 스레드를 계속 깨우는 걸 막는다.
double _round2(double v) => (v * 100.0).roundToDouble() / 100.0;

/// 스피커가 [listener]를 정조준하려면 필요한 yaw(도)와 pitch(도).
///
/// 반환 규약은 `SpeakerNode`와 같다:
/// - yaw 0° = 레이아웃 +y 방향, 시계방향 양수
///   (3D 씬이 로컬 +Z를 정면으로 쓰고 world Z <- 레이아웃 y로 매핑한다)
/// - pitch **음수 = 아래**, 양수 = 위
///
/// `autoAim`이 켜진 스피커는 레이아웃이 바뀔 때마다 이 값으로 갱신된다.
/// 계산만 하고 화면에는 yaw 0을 그리던 예전 방식은, 3D 뷰에서 스피커가
/// 엉뚱한 데를 보고 있어서 엔지니어가 배치를 신뢰할 수 없었다.
({double yawDeg, double pitchDeg}) aimAtListener({
  required double speakerX,
  required double speakerY,
  required double speakerZ,
  required double listenerX,
  required double listenerY,
  required double listenerZ,
}) {
  final double toX = listenerX - speakerX;
  final double toY = listenerY - speakerY;
  final double horiz = math.sqrt(toX * toX + toY * toY);

  // yaw 0이 +y이므로 atan2의 인자 순서가 (x, y)다.
  final double yaw =
      horiz < 1e-6 ? 0.0 : math.atan2(toX, toY) * 180.0 / math.pi;

  // 청취자가 스피커보다 낮으면 아래로 조준해야 하고, 그건 **음수** pitch다.
  // 기존 'Auto-Aim to Listener' 버튼과 같은 식을 쓴다.
  final double pitch =
      math.atan2(listenerZ - speakerZ, math.max(horiz, 1e-6)) * 180.0 / math.pi;

  return (yawDeg: yaw, pitchDeg: pitch);
}
