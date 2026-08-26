import 'dart:math' as math;

class SpeakerNode {
  final String id;
  final double x;
  final double y;
  final int channel;
  final double rotation;
  final double dispersionAngle; // Dispersion H
  final double dispersionAngleV; // Dispersion V
  final double dispersionDistance; // Coverage Distance
  final double heightZ; // Speaker hanging height in meters (e.g. 3.5m)
  final double pitchTilt; // Pitch downward angle in degrees (e.g. 15.0 deg)
  final double maxSPL;
  final double lowCutHz;
  final String boundaryType;
  final double dspLatencyMs;

  SpeakerNode({
    required this.id,
    required this.x,
    required this.y,
    required this.channel,
    this.rotation = 0.0,
    this.dispersionAngle = 90.0,
    this.dispersionAngleV = 90.0,
    this.dispersionDistance = 10.0,
    this.heightZ = 3.5,
    this.pitchTilt = 15.0,
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
    double? x,
    double? y,
    int? channel,
    double? rotation,
    double? dispersionAngle,
    double? dispersionAngleV,
    double? dispersionDistance,
    double? heightZ,
    double? pitchTilt,
    double? maxSPL,
    double? lowCutHz,
    String? boundaryType,
    double? dspLatencyMs,
  }) {
    return SpeakerNode(
      id: id ?? this.id,
      x: x ?? this.x,
      y: y ?? this.y,
      channel: channel ?? this.channel,
      rotation: rotation ?? this.rotation,
      dispersionAngle: dispersionAngle ?? this.dispersionAngle,
      dispersionAngleV: dispersionAngleV ?? this.dispersionAngleV,
      dispersionDistance: dispersionDistance ?? this.dispersionDistance,
      heightZ: heightZ ?? this.heightZ,
      pitchTilt: pitchTilt ?? this.pitchTilt,
      maxSPL: maxSPL ?? this.maxSPL,
      lowCutHz: lowCutHz ?? this.lowCutHz,
      boundaryType: boundaryType ?? this.boundaryType,
      dspLatencyMs: dspLatencyMs ?? this.dspLatencyMs,
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'x': x,
      'y': y,
      'channel': channel,
      'rotation': rotation,
      'dispersion_angle': dispersionAngle,
      'dispersion_distance': dispersionDistance,
      'height_z': heightZ,
      'pitch_tilt': pitchTilt,
    };
  }

  factory SpeakerNode.fromJson(Map<String, dynamic> map) {
    return SpeakerNode(
      id: map['id'],
      x: map['x'].toDouble(),
      y: map['y'].toDouble(),
      channel: map['channel'],
      rotation: (map['rotation'] ?? 0.0).toDouble(),
      dispersionAngle: (map['dispersion_angle'] ?? 90.0).toDouble(),
      dispersionDistance: (map['dispersion_distance'] ?? 220.0).toDouble(),
      heightZ: (map['height_z'] ?? 3.5).toDouble(),
      pitchTilt: (map['pitch_tilt'] ?? 15.0).toDouble(),
    );
  }
}
