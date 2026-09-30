import 'dart:math' as math;

import 'room_zone.dart';

class SpeakerNode {
  final bool isFixed;
  final String id;
  final String? roomId;
  final double x;
  final double y;
  final int channel;
  final double rotation;
  /// 이 스피커를 아직 엔지니어가 조준하지 않았는가(yaw·pitch 공통).
  ///
  /// true면 "청취 지점을 향해 조준한 것으로 본다". 스피커를 놓을 때 yaw
  /// 기본값이 0(레이아웃 +y)이라, 청취자 뒤쪽에 놓기만 해도 스피커가
  /// 등을 돌린 것으로 계산돼 오프액시스 EQ가 고역을 크게 깎았다. 현장에서는
  /// 스피커를 관객 쪽으로 조준하는 게 당연하므로 그쪽을 기본값으로 둔다.
  ///
  /// 인스펙터에서 Yaw나 Pitch를 직접 돌리면 false가 되고, 그때부터는 지정한
  /// 각도를 수평·수직 양쪽 오프액시스 계산에 그대로 쓴다.
  final bool autoAim;
  final double dispersionAngle;
  final double dispersionAngleV;
  final double dispersionDistance;
  final double heightZ; // Speaker hanging height in meters (e.g. 3.5m)
  /// 상하 조준각(도). **음수가 아래**, 양수가 위다.
  ///
  /// 'Auto-Aim to Listener' 버튼이 `atan2(earLevel - heightZ, 수평거리)`로
  /// 계산하므로, 천장에 매단 스피커는 음수가 나온다. 3D 씬의
  /// `rotation.x = -pitchRad`도 같은 규약이다.
  ///
  /// (예전 주석은 "아래로 기운 각도"라고 적혀 있었는데 부호가 반대로 읽혀서,
  /// 그 주석을 믿고 3D 씬 부호를 뒤집었다가 Auto-Aim이 거꾸로 조준한 적이
  /// 있다. 코드가 아니라 주석이 틀렸던 것이다.)
  final double pitchTilt;
  final double panDeg;
  final double reverbSend;
  final double earlyRefMix;
  final double maxSPL;
  /// 스피커 저역 한계(Hz). 자동 EQ가 이 주파수에 보호용 로우컷(12dB/oct)을 건다.
  /// 스피커 인스펙터에서 스피커마다 설정한다. 서브우퍼로 지정하거나 베이스
  /// 매니지먼트로 저역을 서브에 넘기는 동안에는 쓰지 않는다(position_eq.dart).
  final double lowCutHz;
  /// 이 스피커를 자기 방의 서브우퍼로 쓴다(스피커 인스펙터의 Set as LFE Subwoofer).
  /// 방마다 하나다(SpeakerLayoutState.setSubwoofer). 같은 방 메인의 크로스오버 아래
  /// 저역이 이 채널로 오고(rust bass_route.rs), 서브용 자동 FX가 걸린다(position_eq.dart).
  final bool isSubwoofer;
  final String boundaryType;
  final double dspLatencyMs;

  /// 리버브 센드를 0.0~1.0으로 정규화한 값.
  ///
  /// 예전 UI는 0~100(%)로 저장했고 지금 기본값은 0~1이라, 1보다 크면
  /// 퍼센트로 보고 100으로 나눈다. 엔진 전송(speaker_layout_state)과 스피커
  /// 인스펙터의 Send 슬라이더가 이 규칙 하나를 공유해야 화면 표시와 실제로
  /// 걸리는 잔향 양이 어긋나지 않는다.
  double get reverbSendNormalized =>
      (reverbSend > 1.0 ? reverbSend / 100.0 : reverbSend).clamp(0.0, 1.0);

  SpeakerNode({
    required this.id,
    this.roomId,
    required this.x,
    required this.y,
    required this.channel,
    this.isFixed = false,
    this.rotation = 0.0,
    this.autoAim = true,
    this.dispersionAngle = 90.0,
    this.dispersionAngleV = 90.0,
    this.dispersionDistance = 10.0,
    this.heightZ = 3.5,
    this.pitchTilt = 15.0,
    this.panDeg = 0.0,
    this.reverbSend = 0.5,
    // 연출용 초기반사 효과. 기본은 0%다 — 현장 방의 반사는 실제 벽이 만들고,
    // 헤드폰 미리듣기에서는 엔진이 그 방의 반사를 따로 시뮬레이션한다
    // (rust/src/audio/mixer.rs의 refresh_early_ref_mix). 필요할 때만 올려 쓴다.
    this.earlyRefMix = 0.0,
    this.maxSPL = 130.0,
    // 일반 설치형 스피커가 40~60Hz까지 내는 수준. 예전 기본값 80Hz는 서브 없이 쓸 때
    // 저역이 비었다.
    this.lowCutHz = 45.0,
    this.isSubwoofer = false,
    this.boundaryType = 'Free',
    this.dspLatencyMs = 1.2,
  });

  /// Calculates octave frequency dependent dynamic dispersion angle Q(f)
  double getEffectiveDispersionAngle(String octave) {
    switch (octave) {
      case '125Hz':
        return math.min(180.0, dispersionAngle * 2.0); // Omnidirectional low freq
      case '500Hz':
        return math.min(150.0, dispersionAngle * 1.33);
      case '4kHz':
        return dispersionAngle * 0.67; // Narrow high freq beam
      case '1kHz':
      default:
        return dispersionAngle; // Nominal beam angle
    }
  }

  SpeakerNode copyWith({
    String? id,
    String? roomId,
    double? x,
    double? y,
    int? channel,
    double? rotation,
    bool? autoAim,
    double? dispersionAngle,
    bool? isFixed,
    double? dispersionAngleV,
    double? dispersionDistance,
    double? heightZ,
    double? pitchTilt,
    double? panDeg,
    double? reverbSend,
    double? earlyRefMix,
    double? maxSPL,
    double? lowCutHz,
    bool? isSubwoofer,
    String? boundaryType,
    double? dspLatencyMs,
  }) {
    return SpeakerNode(
      id: id ?? this.id,
      roomId: roomId ?? this.roomId,
      x: x ?? this.x,
      y: y ?? this.y,
      channel: channel ?? this.channel,
      rotation: rotation ?? this.rotation,
      autoAim: autoAim ?? this.autoAim,
      dispersionAngle: dispersionAngle ?? this.dispersionAngle,
      isFixed: isFixed ?? this.isFixed,
      dispersionDistance: dispersionDistance ?? this.dispersionDistance,
      heightZ: heightZ ?? this.heightZ,
      pitchTilt: pitchTilt ?? this.pitchTilt,
      panDeg: panDeg ?? this.panDeg,
      reverbSend: reverbSend ?? this.reverbSend,
      earlyRefMix: earlyRefMix ?? this.earlyRefMix,
      dispersionAngleV: dispersionAngleV ?? this.dispersionAngleV,
      maxSPL: maxSPL ?? this.maxSPL,
      lowCutHz: lowCutHz ?? this.lowCutHz,
      isSubwoofer: isSubwoofer ?? this.isSubwoofer,
      boundaryType: boundaryType ?? this.boundaryType,
      dspLatencyMs: dspLatencyMs ?? this.dspLatencyMs,
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'room_id': roomId,
      'x': x,
      'y': y,
      'channel': channel,
      'isFixed': isFixed,
      'rotation': rotation,
      'auto_aim': autoAim,
      'dispersion_angle': dispersionAngle,
      'dispersion_distance': dispersionDistance,
      'height_z': heightZ,
      'pitch_tilt': pitchTilt,
      'pan_deg': panDeg,
      'reverb_send': reverbSend,
      'early_ref_mix': earlyRefMix,
      'dispersion_angle_v': dispersionAngleV,
      'max_spl': maxSPL,
      // 키 이름을 바꿨다. 예전 'low_cut_hz'는 화면에서 바꿀 수 없던 기본값(80Hz)이라
      // 이어받지 않는다(fromJson 참고).
      'low_cut_limit_hz': lowCutHz,
      'is_subwoofer': isSubwoofer,
      'boundary_type': boundaryType,
      'dsp_latency_ms': dspLatencyMs,
    };
  }

  factory SpeakerNode.fromJson(Map<String, dynamic> map) {
    return SpeakerNode(
      id: map['id'],
      roomId: map['room_id'],
      x: map['x'].toDouble(),
      y: map['y'].toDouble(),
      channel: map['channel'],
      isFixed: map['isFixed'] ?? false,
      rotation: (map['rotation'] ?? 0.0).toDouble(),
      // 이 필드가 생기기 전 저장본은 yaw를 의도적으로 돌린 적이 없으므로
      // 자동 조준으로 올린다.
      autoAim: (map['auto_aim'] as bool?) ?? true,
      dispersionAngle: (map['dispersion_angle'] ?? 90.0).toDouble(),
      dispersionDistance: (map['dispersion_distance'] ?? 220.0).toDouble(),
      heightZ: (map['height_z'] ?? 3.5).toDouble(),
      pitchTilt: (map['pitch_tilt'] ?? 15.0).toDouble(),
      panDeg: (map['pan_deg'] ?? 0.0).toDouble(),
      reverbSend: (map['reverb_send'] ?? 0.5).toDouble(),
      earlyRefMix: (map['early_ref_mix'] ?? 0.0).toDouble(),
      dispersionAngleV: (map['dispersion_angle_v'] ?? 90.0).toDouble(),
      maxSPL: (map['max_spl'] ?? 130.0).toDouble(),
      // 예전 'low_cut_hz'는 사용자가 정한 값이 아니라 고정 기본값(80Hz)이었으므로
      // 읽지 않고 새 기본값으로 시작한다. 그대로 이어받으면 기존 스피커만 계속 저역이 잘린다.
      lowCutHz: (map['low_cut_limit_hz'] ?? 45.0).toDouble(),
      // 이 필드가 없는 예전 저장본의 서브 지정(전역 lfeChannel)은 불러올 때
      // SpeakerLayoutState가 옮겨 적는다(adoptLegacyLfeChannel).
      isSubwoofer: (map['is_subwoofer'] as bool?) ?? false,
      boundaryType: map['boundary_type'] ?? 'Free',
      dspLatencyMs: (map['dsp_latency_ms'] ?? 1.2).toDouble(),
    );
  }
}

/// 채널마다 그 채널을 대표하는 스피커 하나를 고른다(채널 → 스피커).
///
/// 현장에서는 채널 하나 = 스피커 하나지만, 출력이 적은 장비로 설계할 때는 같은 채널을
/// 여러 방에 쓰기도 한다(예: 테마 1과 2번 방에 모두 CH2). 예전에는 엔진 좌표는 목록의 첫
/// 스피커, 자동 튜닝은 마지막 스피커 것이 적용되어 한 채널에 서로 다른 방의 값이 섞였다
/// (실기: 테마 1의 CH2에 2번 방 기준 로우컷·극성·게인이 걸림). 이제 **지금 보고 있는 방**
/// ([activeRoomId])의 스피커가 그 채널을 대표하고, 그 방에 없으면 목록의 첫 스피커다.
/// 엔진 전송(spatial_sync_provider)과 자동 튜닝(acoustic_sync_provider)이 이 규칙 하나를
/// 같이 써야 한다.
Map<int, SpeakerNode> channelRepresentatives(
  List<SpeakerNode> nodes,
  String? activeRoomId,
) {
  final representatives = <int, SpeakerNode>{};
  for (final n in nodes) {
    final current = representatives[n.channel];
    if (current == null ||
        (current.roomId != activeRoomId && n.roomId == activeRoomId)) {
      representatives[n.channel] = n;
    }
  }
  return representatives;
}

/// Rust `AudioMixer.channel_positions` (FFI `apiUpdateSpatialConfigJson`)로 보낼
/// `channel_positions` payload를 [nodes]로부터 완전한 필드셋으로 구성한다.
///
/// speaker_layout_state / room_zone_state / trajectory_state 세 Notifier가
/// 모두 동일한 전역 배열을 통째로 덮어쓰기 때문에, 이 헬퍼를 공유해서
/// 서로 다른 스키마로 z/yaw_rotation/pitch_tilt/dispersion_angle이
/// 조용히 0으로 리셋되는 것을 방지한다.
///
/// [node.x]/[node.y]는 캔버스 픽셀 좌표이므로 [pixelsPerMeter]로 나눠 미터로 변환한다.
/// Rust 엔진은 이 좌표로 `SPEED_OF_SOUND_M_S`(343m/s) 기반 시간 정렬 딜레이를 계산하고
/// 미터 단위인 `room_zones` 경계와 직접 비교하므로, 픽셀을 그대로 넘기면 거리가
/// scale배(기본 50배) 부풀려져 딜레이가 수백 ms로 잘못 산출된다.
/// [node.heightZ]는 이미 미터 단위라 변환하지 않는다.
/// 엔진에 보낼 채널별 스피커 좌표. **좌표는 이미 미터이므로 변환하지 않는다.**
///
/// `SpeakerNode.x/y`가 미터라는 근거:
/// - 생성 시 `x: roomWidth * 0.25`(dynamic_3d_room.dart) — roomWidth는 미터.
/// - 3D 룸에서 드래그하면 JS가 `x = newPos.x + room.width/2`로 되돌려준다
///   (studio_engine.html). Three.js 씬은 미터로 만들어진다.
///
/// 예전에는 이 값을 캔버스 픽셀로 오해해 `pixelsPerMeter`(기본 50)로 나눠서
/// 보냈다. 방위각은 균일 배율에 불변이라 티가 안 났지만, 절대 거리가 필요한
/// 계산(초기반사음 경로 길이, 거리 감쇠, 시간 정렬 딜레이)이 전부 50배
/// 작은 값으로 돌아가 사실상 무효였다. 그래서 변환 인자를 아예 제거했다 —
/// 호출부가 실수로 다시 나누지 못하게 하기 위함이다.
///
/// 같은 채널을 여러 방에 쓴 경우 [activeRoomId](지금 보고 있는 방)의 스피커를 보낸다
/// ([channelRepresentatives] 참고). [simBands]가 있으면 스피커마다 헤드폰 미리듣기용
/// 현장 물리 밴드를 `sim_bands`로 싣는다(field_geometry.dart fieldPhysicsPayloadFor).
List<Map<String, dynamic>?> buildChannelPositionsPayload(
  List<SpeakerNode> nodes,
  int channelCount, {
  String? activeRoomId,
  List<Map<String, dynamic>> Function(SpeakerNode node)? simBands,
}) {
  final representatives = channelRepresentatives(nodes, activeRoomId);
  return List.generate(channelCount, (index) {
    final node = representatives[index];
    if (node == null) return null;
    return {
      'x': node.x,
      'y': node.y,
      'z': node.heightZ,
      'yaw_rotation': node.rotation,
      'pitch_tilt': node.pitchTilt,
      'dispersion_angle': node.dispersionAngle,
      'pan_deg': node.panDeg,
      'reverb_send': node.reverbSend,
      'early_ref_mix': node.earlyRefMix,
      // 방마다 로컬 좌표(원점 0,0)라 엔진에서는 RoomZone들이 겹친다. 엔진은 이 값으로
      // 스피커를 자기 방에 연결한다(초기반사·시간 정렬·바이노럴 기준점 등).
      // 방이 없는 예전 스피커는 null → 엔진이 좌표로 찾는다.
      'room_id': node.roomId == null ? null : engineRoomId(node.roomId!),
      // 엔진은 이 값과 room_id로 방별 저역 라우팅 표를 만든다(rust bass_route.rs).
      'is_subwoofer': node.isSubwoofer,
      if (simBands != null) 'sim_bands': simBands(node),
    };
  });
}
