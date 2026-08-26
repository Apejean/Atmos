import re

with open('lib/features/exhibition/models/speaker_node.dart', 'r') as f:
    content = f.read()

target = """class SpeakerNode {
  final String id;
  final double x;
  final double y;
  final int channel;
  final double rotation;
  final double dispersionAngle;
  final double dispersionDistance;
  final double heightZ; // Speaker hanging height in meters (e.g. 3.5m)
  final double pitchTilt; // Pitch downward angle in degrees (e.g. 15.0 deg)

  SpeakerNode({
    required this.id,
    required this.x,
    required this.y,
    required this.channel,
    this.rotation = 0.0,
    this.dispersionAngle = 90.0,
    this.dispersionDistance = 220.0,
    this.heightZ = 3.5,
    this.pitchTilt = 15.0,
  });"""

replacement = """class SpeakerNode {
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
  });"""
  
content = content.replace(target, replacement)

# update copyWith
target_copy = """  SpeakerNode copyWith({
    String? id,
    double? x,
    double? y,
    int? channel,
    double? rotation,
    double? dispersionAngle,
    double? dispersionDistance,
    double? heightZ,
    double? pitchTilt,
  }) {
    return SpeakerNode(
      id: id ?? this.id,
      x: x ?? this.x,
      y: y ?? this.y,
      channel: channel ?? this.channel,
      rotation: rotation ?? this.rotation,
      dispersionAngle: dispersionAngle ?? this.dispersionAngle,
      dispersionDistance: dispersionDistance ?? this.dispersionDistance,
      heightZ: heightZ ?? this.heightZ,
      pitchTilt: pitchTilt ?? this.pitchTilt,
    );
  }"""
  
replacement_copy = """  SpeakerNode copyWith({
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
  }"""
content = content.replace(target_copy, replacement_copy)

with open('lib/features/exhibition/models/speaker_node.dart', 'w') as f:
    f.write(content)
