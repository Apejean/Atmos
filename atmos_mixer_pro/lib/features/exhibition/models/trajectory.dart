class TrajectoryWaypoint {
  final double x;
  final double y;

  const TrajectoryWaypoint(this.x, this.y);

  Map<String, dynamic> toJson() {
    return {
      'x': x,
      'y': y,
    };
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

  const Trajectory({
    required this.id,
    required this.waypoints,
  });

  Trajectory copyWith({
    String? id,
    List<TrajectoryWaypoint>? waypoints,
  }) {
    return Trajectory(
      id: id ?? this.id,
      waypoints: waypoints ?? this.waypoints,
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'waypoints': waypoints.map((w) => w.toJson()).toList(),
    };
  }

  factory Trajectory.fromJson(Map<String, dynamic> json) {
    return Trajectory(
      id: json['id'] as String,
      waypoints: (json['waypoints'] as List)
          .map((w) => TrajectoryWaypoint.fromJson(w as Map<String, dynamic>))
          .toList(),
    );
  }
}
