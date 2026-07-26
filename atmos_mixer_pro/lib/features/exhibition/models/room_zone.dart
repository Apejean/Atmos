class RoomZone {
  final String id;
  final double x;
  final double y;
  final double width;
  final double height;
  final String label;
  final int color;
  final double physicalWidth;
  final double physicalHeight;
  final bool hasDoor;
  final int doorWall; // 0: Top, 1: Right, 2: Bottom, 3: Left
  final double doorOffset; // 0.0 to 1.0

  const RoomZone({
    required this.id,
    required this.x,
    required this.y,
    required this.width,
    required this.height,
    this.label = 'Room',
    this.color = 0xFF2E9A75,
    this.physicalWidth = 5.0,
    this.physicalHeight = 5.0,
    this.hasDoor = false,
    this.doorWall = 0,
    this.doorOffset = 0.5,
  });

  bool containsPoint(double px, double py) {
    return px >= x && px <= x + width && py >= y && py <= y + height;
  }

  RoomZone copyWith({
    String? id,
    double? x,
    double? y,
    double? width,
    double? height,
    String? label,
    int? color,
    double? physicalWidth,
    double? physicalHeight,
    bool? hasDoor,
    int? doorWall,
    double? doorOffset,
  }) {
    return RoomZone(
      id: id ?? this.id,
      x: x ?? this.x,
      y: y ?? this.y,
      width: width ?? this.width,
      height: height ?? this.height,
      label: label ?? this.label,
      color: color ?? this.color,
      physicalWidth: physicalWidth ?? this.physicalWidth,
      physicalHeight: physicalHeight ?? this.physicalHeight,
      hasDoor: hasDoor ?? this.hasDoor,
      doorWall: doorWall ?? this.doorWall,
      doorOffset: doorOffset ?? this.doorOffset,
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
      'hasDoor': hasDoor,
      'doorWall': doorWall,
      'doorOffset': doorOffset,
    };
  }

  factory RoomZone.fromJson(Map<String, dynamic> json) {
    return RoomZone(
      id: json['id'] as String,
      x: (json['x'] as num).toDouble(),
      y: (json['y'] as num).toDouble(),
      width: (json['width'] as num).toDouble(),
      height: (json['height'] as num).toDouble(),
      label: json['label'] as String? ?? 'Room',
      color: json['color'] as int? ?? 0xFF2E9A75,
      physicalWidth: (json['physicalWidth'] as num?)?.toDouble() ?? 5.0,
      physicalHeight: (json['physicalHeight'] as num?)?.toDouble() ?? 5.0,
      hasDoor: json['hasDoor'] as bool? ?? false,
      doorWall: json['doorWall'] as int? ?? 0,
      doorOffset: (json['doorOffset'] as num?)?.toDouble() ?? 0.5,
    );
  }
}
