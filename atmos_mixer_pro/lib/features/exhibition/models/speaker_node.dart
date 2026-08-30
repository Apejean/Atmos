import 'dart:math' as math;

class SpeakerNode {
  final bool isFixed;
  final String id;
  final String? roomId;
  final double x;
  final double y;
  final int channel;
  final double rotation;
  final double dispersionAngle;
  final double dispersionAngleV;
  final double dispersionDistance;
  final double heightZ; // Speaker hanging height in meters (e.g. 3.5m)
  final double pitchTilt; // Pitch downward angle in degrees (e.g. 15.0 deg)
  final double panDeg;
  final double reverbSend;
  final double earlyRefMix;
  final double maxSPL;
  final double lowCutHz;
  final String boundaryType;
  final double dspLatencyMs;

  SpeakerNode({
    required this.id,
    this.roomId,
    required this.x,
    required this.y,
    required this.channel,
    this.isFixed = false,
    this.rotation = 0.0,
    this.dispersionAngle = 90.0,
    this.dispersionAngleV = 90.0,
    this.dispersionDistance = 10.0,
    this.heightZ = 3.5,
    this.pitchTilt = 15.0,
    this.panDeg = 0.0,
    this.reverbSend = 0.5,
    this.earlyRefMix = 0.2,
    this.maxSPL = 130.0,
    this.lowCutHz = 80.0,
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
      'dispersion_angle': dispersionAngle,
      'dispersion_distance': dispersionDistance,
      'height_z': heightZ,
      'pitch_tilt': pitchTilt,
      'pan_deg': panDeg,
      'reverb_send': reverbSend,
      'early_ref_mix': earlyRefMix,
      'dispersion_angle_v': dispersionAngleV,
      'max_spl': maxSPL,
      'low_cut_hz': lowCutHz,
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
      dispersionAngle: (map['dispersion_angle'] ?? 90.0).toDouble(),
      dispersionDistance: (map['dispersion_distance'] ?? 220.0).toDouble(),
      heightZ: (map['height_z'] ?? 3.5).toDouble(),
      pitchTilt: (map['pitch_tilt'] ?? 15.0).toDouble(),
      panDeg: (map['pan_deg'] ?? 0.0).toDouble(),
      reverbSend: (map['reverb_send'] ?? 0.5).toDouble(),
      earlyRefMix: (map['early_ref_mix'] ?? 0.2).toDouble(),
      dispersionAngleV: (map['dispersion_angle_v'] ?? 90.0).toDouble(),
      maxSPL: (map['max_spl'] ?? 130.0).toDouble(),
      lowCutHz: (map['low_cut_hz'] ?? 80.0).toDouble(),
      boundaryType: map['boundary_type'] ?? 'Free',
      dspLatencyMs: (map['dsp_latency_ms'] ?? 1.2).toDouble(),
    );
  }
}
