import 'dart:math' as math;

class RoomMaterialPreset {
  final String name;
  final List<double> alphaOctaves; // [125Hz, 250Hz, 500Hz, 1kHz, 2kHz, 4kHz]

  const RoomMaterialPreset(this.name, this.alphaOctaves);

  double get averageAlpha => alphaOctaves.reduce((a, b) => a + b) / alphaOctaves.length;

  static const List<RoomMaterialPreset> presets = [
    RoomMaterialPreset('Concrete / Tile (Hard Reflective)', [0.01, 0.01, 0.02, 0.02, 0.02, 0.03]),
    RoomMaterialPreset('Drywall / Glass (Standard Indoor)', [0.15, 0.12, 0.10, 0.08, 0.07, 0.06]),
    RoomMaterialPreset('Wood Panel (Warm Resonance)', [0.28, 0.22, 0.18, 0.14, 0.12, 0.10]),
    RoomMaterialPreset('Carpet & Heavy Curtain (Medium Absorber)', [0.08, 0.15, 0.28, 0.38, 0.45, 0.50]),
    RoomMaterialPreset('Acoustic Foam / Panel (High Absorber)', [0.12, 0.35, 0.70, 0.85, 0.92, 0.95]),
  ];
}

class RoomZone {
  final String id;
  final String label;
  final double x;
  final double y;
  final double width;
  final double height;
  final int color;
  final double physicalWidth;
  final double physicalHeight;
  final bool hasDoor;
  final int doorWall;
  final double doorOffset;
  final double rotation;
  final String materialName;
  final double absorptionCoeff;
  final List<double> alphaOctaves;
  final double wallTransmissionLoss; // STC / TL in dB (e.g. 35 dB)

  const RoomZone({
    required this.id,
    this.label = 'Room',
    required this.x,
    required this.y,
    required this.width,
    required this.height,
    required this.color,
    this.physicalWidth = 5.0,
    this.physicalHeight = 5.0,
    this.hasDoor = false,
    this.doorWall = 0,
    this.doorOffset = 0.5,
    this.rotation = 0.0,
    this.materialName = 'Drywall / Glass (Standard Indoor)',
    this.absorptionCoeff = 0.10,
    this.alphaOctaves = const [0.15, 0.12, 0.10, 0.08, 0.07, 0.06],
    this.wallTransmissionLoss = 35.0,
  });

  /// Eyring + Sabine Hybrid Reverberation Engine
  /// Switched to Eyring when average alpha >= 0.2 for dead/absorptive rooms (ISO 3382-1)
  double get estimatedRt60 {
    final heightM = 3.0; // Standard room height assumption (3m)
    final volume = physicalWidth * physicalHeight * heightM;
    final surfaceArea = 2 * (physicalWidth * physicalHeight + physicalWidth * heightM + physicalHeight * heightM);
    final avgAlpha = absorptionCoeff.clamp(0.01, 0.99);

    if (avgAlpha >= 0.20) {
      // Eyring Formula for absorptive / dead acoustic rooms
      final airAbsorption = 0.002 * volume; // 4*m*V approximation
      final denominator = -surfaceArea * math.log(1.0 - avgAlpha) + airAbsorption;
      if (denominator <= 0) return 0.2;
      return (0.161 * volume) / denominator;
    } else {
      // Sabine Formula for reflective / live rooms
      final totalAbsorption = surfaceArea * avgAlpha;
      if (totalAbsorption <= 0) return 0.5;
      return (0.161 * volume) / totalAbsorption;
    }
  }

  bool containsPoint(double px, double py) {
    if (rotation == 0.0) {
      return px >= x && px <= x + width && py >= y && py <= y + height;
    }

    final double cx = x + width / 2;
    final double cy = y + height / 2;

    // Inverse rotation
    final double rad = -rotation * math.pi / 180.0;
    final double cosTheta = math.cos(rad);
    final double sinTheta = math.sin(rad);

    // Translate point to origin (center)
    final double translatedX = px - cx;
    final double translatedY = py - cy;

    // Rotate point
    final double rotatedX = translatedX * cosTheta - translatedY * sinTheta;
    final double rotatedY = translatedX * sinTheta + translatedY * cosTheta;

    // Translate back and check AABB bounds
    final double finalX = rotatedX + cx;
    final double finalY = rotatedY + cy;

    return finalX >= x && finalX <= x + width && finalY >= y && finalY <= y + height;
  }

  RoomZone copyWith({
    String? id,
    String? label,
    double? x,
    double? y,
    double? width,
    double? height,
    int? color,
    double? physicalWidth,
    double? physicalHeight,
    bool? hasDoor,
    int? doorWall,
    double? doorOffset,
    double? rotation,
    String? materialName,
    double? absorptionCoeff,
    List<double>? alphaOctaves,
    double? wallTransmissionLoss,
  }) {
    return RoomZone(
      id: id ?? this.id,
      label: label ?? this.label,
      x: x ?? this.x,
      y: y ?? this.y,
      width: width ?? this.width,
      height: height ?? this.height,
      color: color ?? this.color,
      physicalWidth: physicalWidth ?? this.physicalWidth,
      physicalHeight: physicalHeight ?? this.physicalHeight,
      hasDoor: hasDoor ?? this.hasDoor,
      doorWall: doorWall ?? this.doorWall,
      doorOffset: doorOffset ?? this.doorOffset,
      rotation: rotation ?? this.rotation,
      materialName: materialName ?? this.materialName,
      absorptionCoeff: absorptionCoeff ?? this.absorptionCoeff,
      alphaOctaves: alphaOctaves ?? this.alphaOctaves,
      wallTransmissionLoss: wallTransmissionLoss ?? this.wallTransmissionLoss,
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'x': x,
      'y': y,
      'width': width,
      'height': height,
      'label': label,
      'color': color,
      'physicalWidth': physicalWidth,
      'physicalHeight': physicalHeight,
      'has_door': hasDoor,
      'door_wall': doorWall,
      'door_offset': doorOffset,
      'rotation': rotation,
      'material_name': materialName,
      'absorption_coeff': absorptionCoeff,
      'alpha_octaves': alphaOctaves,
      'wall_transmission_loss': wallTransmissionLoss,
    };
  }

  factory RoomZone.fromJson(Map<String, dynamic> json) {
    return RoomZone(
      id: json['id'] as String,
      label: json['label'] as String? ?? 'Room',
      x: (json['x'] as num).toDouble(),
      y: (json['y'] as num).toDouble(),
      width: (json['width'] as num).toDouble(),
      height: (json['height'] as num).toDouble(),
      color: json['color'] as int,
      physicalWidth: (json['physical_width'] as num?)?.toDouble() ?? 5.0,
      physicalHeight: (json['physical_height'] as num?)?.toDouble() ?? 5.0,
      hasDoor: json['has_door'] as bool? ?? false,
      doorWall: json['door_wall'] as int? ?? 0,
      doorOffset: (json['door_offset'] as num?)?.toDouble() ?? 0.5,
      rotation: (json['rotation'] as num?)?.toDouble() ?? 0.0,
      materialName: json['material_name'] as String? ?? 'Drywall / Glass (Standard Indoor)',
      absorptionCoeff: (json['absorption_coeff'] as num?)?.toDouble() ?? 0.10,
      alphaOctaves: (json['alpha_octaves'] as List<dynamic>?)?.map((e) => (e as num).toDouble()).toList() ?? const [0.15, 0.12, 0.10, 0.08, 0.07, 0.06],
      wallTransmissionLoss: (json['wall_transmission_loss'] as num?)?.toDouble() ?? 35.0,
    );
  }
}
