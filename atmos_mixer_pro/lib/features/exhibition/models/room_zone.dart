import 'dart:math' as math;

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
  });

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
    );
  }
}
