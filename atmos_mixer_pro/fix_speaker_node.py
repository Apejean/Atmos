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
  final double panDeg;
  final double reverbSend;
  final double earlyRefMix;"""

replacement = """class SpeakerNode {
  final String id;
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
  final double dspLatencyMs;"""
content = content.replace(target, replacement)

target2 = """  SpeakerNode({
    required this.id,
    required this.x,
    required this.y,
    required this.channel,
    this.rotation = 0.0,
    this.dispersionAngle = 90.0,
    this.dispersionDistance = 220.0,
    this.heightZ = 3.5,
    this.pitchTilt = 15.0,
    this.panDeg = 0.0,
    this.reverbSend = 0.5,
    this.earlyRefMix = 0.2,
  });"""

replacement2 = """  SpeakerNode({
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
    this.panDeg = 0.0,
    this.reverbSend = 0.5,
    this.earlyRefMix = 0.2,
    this.maxSPL = 130.0,
    this.lowCutHz = 80.0,
    this.boundaryType = 'Free',
    this.dspLatencyMs = 1.2,
  });"""
content = content.replace(target2, replacement2)

target3 = """  SpeakerNode copyWith({
    String? id,
    double? x,
    double? y,
    int? channel,
    double? rotation,
    double? dispersionAngle,
    double? dispersionDistance,
    double? heightZ,
    double? pitchTilt,
    double? panDeg,
    double? reverbSend,
    double? earlyRefMix,
  }) {"""
  
replacement3 = """  SpeakerNode copyWith({
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
    double? panDeg,
    double? reverbSend,
    double? earlyRefMix,
    double? maxSPL,
    double? lowCutHz,
    String? boundaryType,
    double? dspLatencyMs,
  }) {"""
content = content.replace(target3, replacement3)

target4 = """      heightZ: heightZ ?? this.heightZ,
      pitchTilt: pitchTilt ?? this.pitchTilt,
      panDeg: panDeg ?? this.panDeg,
      reverbSend: reverbSend ?? this.reverbSend,
      earlyRefMix: earlyRefMix ?? this.earlyRefMix,
    );"""

replacement4 = """      heightZ: heightZ ?? this.heightZ,
      pitchTilt: pitchTilt ?? this.pitchTilt,
      panDeg: panDeg ?? this.panDeg,
      reverbSend: reverbSend ?? this.reverbSend,
      earlyRefMix: earlyRefMix ?? this.earlyRefMix,
      dispersionAngleV: dispersionAngleV ?? this.dispersionAngleV,
      maxSPL: maxSPL ?? this.maxSPL,
      lowCutHz: lowCutHz ?? this.lowCutHz,
      boundaryType: boundaryType ?? this.boundaryType,
      dspLatencyMs: dspLatencyMs ?? this.dspLatencyMs,
    );"""
content = content.replace(target4, replacement4)

target5 = """      'height_z': heightZ,
      'pitch_tilt': pitchTilt,
      'pan_deg': panDeg,
      'reverb_send': reverbSend,
      'early_ref_mix': earlyRefMix,
    };"""

replacement5 = """      'height_z': heightZ,
      'pitch_tilt': pitchTilt,
      'pan_deg': panDeg,
      'reverb_send': reverbSend,
      'early_ref_mix': earlyRefMix,
      'dispersion_angle_v': dispersionAngleV,
      'max_spl': maxSPL,
      'low_cut_hz': lowCutHz,
      'boundary_type': boundaryType,
      'dsp_latency_ms': dspLatencyMs,
    };"""
content = content.replace(target5, replacement5)

target6 = """      heightZ: (map['height_z'] ?? 3.5).toDouble(),
      pitchTilt: (map['pitch_tilt'] ?? 15.0).toDouble(),
      panDeg: (map['pan_deg'] ?? 0.0).toDouble(),
      reverbSend: (map['reverb_send'] ?? 0.5).toDouble(),
      earlyRefMix: (map['early_ref_mix'] ?? 0.2).toDouble(),
    );"""

replacement6 = """      heightZ: (map['height_z'] ?? 3.5).toDouble(),
      pitchTilt: (map['pitch_tilt'] ?? 15.0).toDouble(),
      panDeg: (map['pan_deg'] ?? 0.0).toDouble(),
      reverbSend: (map['reverb_send'] ?? 0.5).toDouble(),
      earlyRefMix: (map['early_ref_mix'] ?? 0.2).toDouble(),
      dispersionAngleV: (map['dispersion_angle_v'] ?? 90.0).toDouble(),
      maxSPL: (map['max_spl'] ?? 130.0).toDouble(),
      lowCutHz: (map['low_cut_hz'] ?? 80.0).toDouble(),
      boundaryType: map['boundary_type'] ?? 'Free',
      dspLatencyMs: (map['dsp_latency_ms'] ?? 1.2).toDouble(),
    );"""
content = content.replace(target6, replacement6)

with open('lib/features/exhibition/models/speaker_node.dart', 'w') as f:
    f.write(content)
