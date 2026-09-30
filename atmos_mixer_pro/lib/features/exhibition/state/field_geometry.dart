import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/position_eq.dart';

/// 스피커 한 대가 속한 방의 치수와 청취 지점(방 정중앙, 귀 높이).
///
/// 자동 튜닝(acoustic_sync_provider)과 헤드폰 미리듣기의 현장 물리 밴드(spatial_sync_provider)가
/// **같은 방·같은 청취 지점**을 봐야 보정과 물리가 정확히 상쇄된다. 그래서 한 곳에서 정한다.
class FieldGeometry {
  final double roomWidth;
  final double roomDepth;
  final double ceilingHeight;
  final double listenerX;
  final double listenerY;
  final double listenerZ;
  final double absorption;

  const FieldGeometry({
    required this.roomWidth,
    required this.roomDepth,
    required this.ceilingHeight,
    required this.listenerX,
    required this.listenerY,
    required this.listenerZ,
    required this.absorption,
  });
}

/// [speaker]의 방(없으면 첫 방, 방이 하나도 없으면 청사진 캔버스)과 그 방 중앙의 청취 지점.
FieldGeometry fieldGeometryFor(
  SpeakerNode speaker,
  List<RoomZone> rooms,
  BlueprintData blueprint,
) {
  // 방을 따로 만들지 않고 청사진 캔버스 위에서만 작업하는 경우가 있다(3D 뷰와
  // Auto-Aim이 그 상황을 캔버스 치수로 폴백해 지원한다).
  final room = rooms.where((r) => r.id == speaker.roomId).firstOrNull ??
      (rooms.isNotEmpty ? rooms.first : null);
  final double width = room?.physicalWidth ?? blueprint.canvasWidthMeters;
  final double depth = room?.physicalHeight ?? blueprint.canvasHeightMeters;
  return FieldGeometry(
    roomWidth: width,
    roomDepth: depth,
    ceilingHeight: room?.ceilingHeight ?? 3.0,
    listenerX: width / 2.0,
    listenerY: depth / 2.0,
    listenerZ: room?.earLevel ?? 1.2,
    absorption: room?.absorptionCoeff ?? 0.3,
  );
}

/// 헤드폰 미리듣기에 보낼 [speaker]의 현장 물리 밴드(엔진 payload 형식).
///
/// 형식은 Rust `api_update_spatial_config_json`의 `sim_bands`와 같다:
/// `{"on", "type"(EqType 순서 번호), "freq", "gain", "q"}` x [kFieldPhysicsBandCount] — 슬롯 고정.
List<Map<String, dynamic>> fieldPhysicsPayloadFor(
  SpeakerNode speaker,
  FieldGeometry g,
  double speedOfSound,
) {
  final bands = computeFieldPhysicsBands(
    speakerX: speaker.x,
    speakerY: speaker.y,
    speakerZ: speaker.heightZ,
    listenerX: g.listenerX,
    listenerY: g.listenerY,
    listenerZ: g.listenerZ,
    roomWidth: g.roomWidth,
    roomDepth: g.roomDepth,
    ceilingHeight: g.ceilingHeight,
    yawDeg: speaker.rotation,
    autoAim: speaker.autoAim,
    dispersionAngleDeg: speaker.dispersionAngle,
    dispersionAngleVDeg: speaker.dispersionAngleV,
    pitchTiltDeg: speaker.pitchTilt,
    speedOfSound: speedOfSound,
    absorptionCoeff: g.absorption,
    isSubwoofer: speaker.isSubwoofer,
  );
  return [
    for (final b in bands)
      {'on': b.enabled, 'type': b.type.index, 'freq': b.freq, 'gain': b.gain, 'q': b.q},
  ];
}
