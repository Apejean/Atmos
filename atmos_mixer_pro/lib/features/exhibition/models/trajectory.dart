class TrajectoryWaypoint {
  final double x;
  final double y;

  const TrajectoryWaypoint(this.x, this.y);

  Map<String, dynamic> toJson() {
    return {'x': x, 'y': y};
  }

  factory TrajectoryWaypoint.fromJson(Map<String, dynamic> json) {
    return TrajectoryWaypoint(
      (json['x'] as num).toDouble(),
      (json['y'] as num).toDouble(),
    );
  }
}

class Trajectory {
  final String id;
  final List<TrajectoryWaypoint> waypoints;
  final double speed;
  final bool isPingPong;
  final bool isVisible;

  const Trajectory({
    required this.id,
    required this.waypoints,
    this.speed = 1.0,
    this.isPingPong = false,
    this.isVisible = true,
  });

  Trajectory copyWith({
    String? id,
    List<TrajectoryWaypoint>? waypoints,
    double? speed,
    bool? isPingPong,
    bool? isVisible,
  }) {
    return Trajectory(
      id: id ?? this.id,
      waypoints: waypoints ?? this.waypoints,
      speed: speed ?? this.speed,
      isPingPong: isPingPong ?? this.isPingPong,
      isVisible: isVisible ?? this.isVisible,
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'waypoints': waypoints.map((w) => w.toJson()).toList(),
      'speed': speed,
      'isPingPong': isPingPong,
      'isVisible': isVisible,
    };
  }

  factory Trajectory.fromJson(Map<String, dynamic> json) {
    return Trajectory(
      id: json['id'] as String,
      waypoints: (json['waypoints'] as List)
          .map((w) => TrajectoryWaypoint.fromJson(w as Map<String, dynamic>))
          .toList(),
      speed: (json['speed'] as num?)?.toDouble() ?? 1.0,
      isPingPong: json['isPingPong'] as bool? ?? false,
      isVisible: json['isVisible'] as bool? ?? true,
    );
  }
}
