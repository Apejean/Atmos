import 'dart:ui';
import 'package:flutter/foundation.dart';
import 'dart:math' as math;

class Waypoint {
  // Note: For Room-Bound trajectories, position is a relative coordinate (0.0 ~ 1.0)
  // based on the target room's dimensions. For global trajectories, it may represent
  // absolute meters or normalized canvas coordinates depending on usage.
  final Offset position;
  final double heightZ;

  const Waypoint({required this.position, this.heightZ = 1.5});

  Map<String, dynamic> toJson() {
    return {'x': position.dx, 'y': position.dy, 'z': heightZ};
  }

  factory Waypoint.fromJson(Map<String, dynamic> json) {
    return Waypoint(
      position: Offset((json['x'] as num).toDouble(), (json['y'] as num).toDouble()),
      heightZ: (json['z'] as num?)?.toDouble() ?? 1.5,
    );
  }
}

class TrajectoryModel extends ChangeNotifier {
  final String id;
  String name;
  Color color;
  List<Waypoint> waypoints;

  double speed;
  bool isPingPong;
  bool isVisible;
  String? audioFilePath;
  String? audioTrackId;
  String? stemGroupId;
  String? targetRoomZoneId;
  double size;

  double progress = 0.0;
  double direction = 1.0;

  TrajectoryModel({
    required this.id,
    this.name = 'Trajectory',
    this.color = const Color(0xFF00F2FE),
    required this.waypoints,
    this.speed = 2.0,
    this.isPingPong = false,
    this.isVisible = true,
    this.audioFilePath,
    this.audioTrackId,
    this.stemGroupId,
    this.targetRoomZoneId,
    this.size = 0.2,
  });

  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'name': name,
      'color': color.toARGB32(),
      'waypoints': waypoints.map((w) => w.toJson()).toList(),
      'speed': speed,
      'isPingPong': isPingPong,
      'isVisible': isVisible,
      'audioFilePath': audioFilePath,
      'audioTrackId': audioTrackId,
      'stemGroupId': stemGroupId,
      'targetRoomZoneId': targetRoomZoneId,
      'size': size,
    };
  }

  factory TrajectoryModel.fromJson(Map<String, dynamic> json) {
    return TrajectoryModel(
      id: json['id'] as String,
      name: json['name'] as String? ?? 'Trajectory',
      color: json['color'] != null ? Color(json['color'] as int) : const Color(0xFF00F2FE),
      waypoints: (json['waypoints'] as List)
          .map((w) => Waypoint.fromJson(w as Map<String, dynamic>))
          .toList(),
      speed: (json['speed'] as num?)?.toDouble() ?? 2.0,
      isPingPong: json['isPingPong'] as bool? ?? false,
      isVisible: json['isVisible'] as bool? ?? true,
      audioFilePath: json['audioFilePath'] as String?,
      audioTrackId: json['audioTrackId'] as String?,
      stemGroupId: json['stemGroupId'] as String?,
      targetRoomZoneId: json['targetRoomZoneId'] as String?,
      size: (json['size'] as num?)?.toDouble() ?? 0.2,
    );
  }

  double get totalPathLength {
    if (waypoints.length < 2) return 0;
    double len = 0;
    for (int i = 0; i < waypoints.length - 1; i++) {
      len += (waypoints[i + 1].position - waypoints[i].position).distance;
    }
    return len;
  }

  void updateProgress(double deltaTime, double totalLength) {
    if (waypoints.length < 2 || totalLength <= 0) return;
    double deltaProgress = (speed * deltaTime) / totalLength;

    if (isPingPong) {
      progress += deltaProgress * direction;
      if (progress >= 1.0) {
        progress = 1.0;
        direction = -1.0;
      } else if (progress <= 0.0) {
        progress = 0.0;
        direction = 1.0;
      }
    } else {
      progress = (progress + deltaProgress) % 1.0;
    }
    notifyListeners();
  }

  Offset getCurrentPositionMeter() {
    return _getPointOnPath(progress);
  }

  double getCurrentHeightZ() {
    if (waypoints.isEmpty) return 1.5;
    if (waypoints.length == 1) return waypoints.first.heightZ;
    
    double t = progress * (waypoints.length - 1);
    int index = t.floor();
    if (index >= waypoints.length - 1) return waypoints.last.heightZ;
    double localT = t - index;
    return waypoints[index].heightZ + (waypoints[index + 1].heightZ - waypoints[index].heightZ) * localT;
  }

  Offset _getPointOnPath(double t) {
    if (waypoints.isEmpty) return Offset.zero;
    if (waypoints.length == 1) return waypoints.first.position;
    if (waypoints.length == 2) {
      return Offset.lerp(waypoints.first.position, waypoints.last.position, t)!;
    }

    double scaledT = t * (waypoints.length - 1);
    int p1 = scaledT.floor();
    int p2 = p1 + 1;
    double localT = scaledT - p1;

    if (p2 >= waypoints.length) {
      return waypoints.last.position;
    }

    int p0 = math.max(0, p1 - 1);
    int p3 = math.min(waypoints.length - 1, p2 + 1);

    return _calculateCatmullRom(
        waypoints[p0].position,
        waypoints[p1].position,
        waypoints[p2].position,
        waypoints[p3].position,
        localT);
  }

  Offset _calculateCatmullRom(Offset p0, Offset p1, Offset p2, Offset p3, double t) {
    double t2 = t * t;
    double t3 = t2 * t;
    double x = 0.5 * ((2 * p1.dx) +
        (-p0.dx + p2.dx) * t +
        (2 * p0.dx - 5 * p1.dx + 4 * p2.dx - p3.dx) * t2 +
        (-p0.dx + 3 * p1.dx - 3 * p2.dx + p3.dx) * t3);
    double y = 0.5 * ((2 * p1.dy) +
        (-p0.dy + p2.dy) * t +
        (2 * p0.dy - 5 * p1.dy + 4 * p2.dy - p3.dy) * t2 +
        (-p0.dy + 3 * p1.dy - 3 * p2.dy + p3.dy) * t3);
    return Offset(x, y);
  }

  void markNeedsRepaint() {
    notifyListeners();
  }

  TrajectoryModel copyWith({
    String? id,
    String? name,
    Color? color,
    List<Waypoint>? waypoints,
    double? speed,
    bool? isPingPong,
    bool? isVisible,
    String? audioFilePath,
    String? audioTrackId,
    String? stemGroupId,
    String? targetRoomZoneId,
    double? size,
  }) {
    return TrajectoryModel(
      id: id ?? this.id,
      name: name ?? this.name,
      color: color ?? this.color,
      waypoints: waypoints ?? List.from(this.waypoints),
      speed: speed ?? this.speed,
      isPingPong: isPingPong ?? this.isPingPong,
      isVisible: isVisible ?? this.isVisible,
      audioFilePath: audioFilePath ?? this.audioFilePath,
      audioTrackId: audioTrackId ?? this.audioTrackId,
      stemGroupId: stemGroupId ?? this.stemGroupId,
      targetRoomZoneId: targetRoomZoneId ?? this.targetRoomZoneId,
      size: size ?? this.size,
    );
  }
}
