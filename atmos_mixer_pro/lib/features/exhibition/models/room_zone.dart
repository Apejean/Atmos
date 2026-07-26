class RoomZone {
  final String id;
  final double x;
  final double y;
  final double width;
  final double height;
  final String label;
  final int color;

  const RoomZone({
    required this.id,
    required this.x,
    required this.y,
    required this.width,
    required this.height,
    this.label = 'Room',
    this.color = 0xFF2E9A75,
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
  }) {
    return RoomZone(
      id: id ?? this.id,
      x: x ?? this.x,
      y: y ?? this.y,
      width: width ?? this.width,
      height: height ?? this.height,
      label: label ?? this.label,
      color: color ?? this.color,
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
    );
  }
}
