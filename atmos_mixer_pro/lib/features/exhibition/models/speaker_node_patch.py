import re

with open('lib/features/exhibition/models/speaker_node.dart', 'r') as f:
    content = f.read()

# Add new fields
content = re.sub(
    r'(final double pitchTilt;.*)',
    r'\1\n  final double panDeg;\n  final double maxSpl;\n  final double dispersionH;\n  final double dispersionV;\n  final double lowCutHz;\n  final String boundaryType;\n  final double internalLatencyMs;\n  final double coverageDistance;',
    content
)

# Add to constructor
content = re.sub(
    r'(this\.pitchTilt = 15\.0,)',
    r"\1\n    this.panDeg = 0.0,\n    this.maxSpl = 122.0,\n    this.dispersionH = 90.0,\n    this.dispersionV = 60.0,\n    this.lowCutHz = 45.0,\n    this.boundaryType = 'Free',\n    this.internalLatencyMs = 1.5,\n    this.coverageDistance = 15.0,",
    content
)

# Update copyWith
content = re.sub(
    r'(double\? pitchTilt,)',
    r'\1\n    double? panDeg,\n    double? maxSpl,\n    double? dispersionH,\n    double? dispersionV,\n    double? lowCutHz,\n    String? boundaryType,\n    double? internalLatencyMs,\n    double? coverageDistance,',
    content
)

content = re.sub(
    r'(pitchTilt: pitchTilt \?\? this\.pitchTilt,)',
    r'\1\n      panDeg: panDeg ?? this.panDeg,\n      maxSpl: maxSpl ?? this.maxSpl,\n      dispersionH: dispersionH ?? this.dispersionH,\n      dispersionV: dispersionV ?? this.dispersionV,\n      lowCutHz: lowCutHz ?? this.lowCutHz,\n      boundaryType: boundaryType ?? this.boundaryType,\n      internalLatencyMs: internalLatencyMs ?? this.internalLatencyMs,\n      coverageDistance: coverageDistance ?? this.coverageDistance,',
    content
)

# Update toJson
content = re.sub(
    r"('pitch_tilt': pitchTilt,)",
    r"\1\n      'pan_deg': panDeg,\n      'max_spl': maxSpl,\n      'dispersion_h': dispersionH,\n      'dispersion_v': dispersionV,\n      'low_cut_hz': lowCutHz,\n      'boundary_type': boundaryType,\n      'internal_latency_ms': internalLatencyMs,\n      'coverage_distance': coverageDistance,",
    content
)

# Update fromJson
content = re.sub(
    r"(pitchTilt: \(map\['pitch_tilt'\] \?\? 15\.0\)\.toDouble\(\),)",
    r"\1\n      panDeg: (map['pan_deg'] ?? 0.0).toDouble(),\n      maxSpl: (map['max_spl'] ?? 122.0).toDouble(),\n      dispersionH: (map['dispersion_h'] ?? 90.0).toDouble(),\n      dispersionV: (map['dispersion_v'] ?? 60.0).toDouble(),\n      lowCutHz: (map['low_cut_hz'] ?? 45.0).toDouble(),\n      boundaryType: map['boundary_type'] ?? 'Free',\n      internalLatencyMs: (map['internal_latency_ms'] ?? 1.5).toDouble(),\n      coverageDistance: (map['coverage_distance'] ?? 15.0).toDouble(),",
    content
)

with open('lib/features/exhibition/models/speaker_node.dart', 'w') as f:
    f.write(content)
