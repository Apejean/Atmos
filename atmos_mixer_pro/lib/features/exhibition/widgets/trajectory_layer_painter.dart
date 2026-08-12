import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/trajectory.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';

class TrajectoryLayerPainter extends CustomPainter {
  final List<TrajectoryModel> trajectories;
  final List<RoomZone> rooms;
  final List<SpeakerNode> speakers;
  final String? focusedTrajectoryId;
  final double scaleMeterToPixel;

  TrajectoryLayerPainter({
    required this.trajectories,
    required this.rooms,
    required this.speakers,
    required this.focusedTrajectoryId,
    required this.scaleMeterToPixel,
    required Listenable repaint,
  }) : super(repaint: repaint);

  @override
  void paint(Canvas canvas, Size size) {
    for (var traj in trajectories) {
      if (!traj.isVisible) continue;

      bool isFocused = (focusedTrajectoryId == null) || (traj.id == focusedTrajectoryId);
      
      double opacity = isFocused ? 1.0 : 0.15;
      double strokeWidth = isFocused ? 3.5 : 1.0;

      final pathPaint = Paint()
        ..color = traj.color.withValues(alpha: opacity)
        ..style = PaintingStyle.stroke
        ..strokeWidth = strokeWidth
        ..strokeCap = StrokeCap.round;

      Path splinePath = _buildSplinePath(traj);

      if (isFocused && focusedTrajectoryId != null) {
        final glowPaint = Paint()
          ..color = traj.color.withValues(alpha: 0.6)
          ..style = PaintingStyle.stroke
          ..strokeWidth = strokeWidth + 4.0
          ..maskFilter = const MaskFilter.blur(BlurStyle.solid, 8.0);
        canvas.drawPath(splinePath, glowPaint);
      }

      canvas.drawPath(splinePath, pathPaint);

      Offset currentPos = _getAbsoluteOffset(traj.getCurrentPositionMeter(), traj);
      final nodePaint = Paint()
        ..color = isFocused ? Colors.white : traj.color.withValues(alpha: opacity)
        ..style = PaintingStyle.fill;

      canvas.drawCircle(currentPos, isFocused ? 8.0 : 4.0, nodePaint);
      
      // Draw waypoints
      if (isFocused) {
        final waypointPaint = Paint()
          ..color = traj.color
          ..style = PaintingStyle.fill;
        final waypointBorder = Paint()
          ..color = Colors.white
          ..style = PaintingStyle.stroke
          ..strokeWidth = 2.0;
          
        for (var wp in traj.waypoints) {
          Offset p = _getAbsoluteOffset(wp.position, traj);
          canvas.drawCircle(p, 6.0, waypointPaint);
          canvas.drawCircle(p, 6.0, waypointBorder);
        }

        // Draw Dynamic Gain Links & Halos for target room speakers
        if (traj.targetRoomZoneId != null) {
          final targetRoom = rooms.where((r) => r.id == traj.targetRoomZoneId).firstOrNull;
          if (targetRoom != null) {
            for (var speaker in speakers) {
              // speaker.x, speaker.y are top-left, center is +30, +30 (assuming 60 size)
              final speakerCenter = Offset(speaker.x + 30.0, speaker.y + 30.0);
              if (targetRoom.containsPoint(speakerCenter.dx, speakerCenter.dy)) {
                final dist = (speakerCenter - currentPos).distance;
                // Basic inverse-distance attenuation approximation (clamped max width)
                final strength = (100.0 / (dist + 1.0)).clamp(0.0, 1.0);
                
                // Dynamic Gain Link (Laser)
                final linkPaint = Paint()
                  ..color = traj.color.withValues(alpha: strength * 0.8)
                  ..style = PaintingStyle.stroke
                  ..strokeWidth = 1.0 + (strength * 3.0);
                canvas.drawLine(currentPos, speakerCenter, linkPaint);

                // Speaker Halo (Glow)
                if (strength > 0.1) {
                  final haloPaint = Paint()
                    ..color = traj.color.withValues(alpha: strength * 0.4)
                    ..style = PaintingStyle.fill
                    ..maskFilter = const MaskFilter.blur(BlurStyle.normal, 12.0);
                  canvas.drawCircle(speakerCenter, 40.0, haloPaint);
                }
              }
            }
          }
        }
      }
    }
  }

  Path _buildSplinePath(TrajectoryModel traj) {
    final path = Path();
    if (traj.waypoints.isEmpty) return path;
    
    if (traj.waypoints.length == 1) {
      Offset p = _getAbsoluteOffset(traj.waypoints.first.position, traj);
      path.moveTo(p.dx, p.dy);
      path.lineTo(p.dx, p.dy);
      return path;
    }
    
    if (traj.waypoints.length == 2) {
      Offset p0 = _getAbsoluteOffset(traj.waypoints.first.position, traj);
      Offset p1 = _getAbsoluteOffset(traj.waypoints.last.position, traj);
      path.moveTo(p0.dx, p0.dy);
      path.lineTo(p1.dx, p1.dy);
      return path;
    }
    
    // For Catmull-Rom we sample points along the path
    Offset start = _getAbsoluteOffset(traj.waypoints.first.position, traj);
    path.moveTo(start.dx, start.dy);
    
    int resolution = traj.waypoints.length * 20; // 20 samples per segment
    for (int i = 1; i <= resolution; i++) {
      double t = i / resolution;
      // We need a helper method to get the point based on t (0.0 to 1.0) for the whole path
      // Actually we can just temporarily use the model's getCurrentPositionMeter for sampling
      // but since getCurrentPositionMeter relies on progress, we can add a method or compute here.
      double scaledT = t * (traj.waypoints.length - 1);
      int p1 = scaledT.floor();
      int p2 = p1 + 1;
      double localT = scaledT - p1;

      if (p2 >= traj.waypoints.length) {
        Offset lastP = _getAbsoluteOffset(traj.waypoints.last.position, traj);
        path.lineTo(lastP.dx, lastP.dy);
        continue;
      }

      int p0 = (p1 - 1).clamp(0, traj.waypoints.length - 1);
      int p3 = (p2 + 1).clamp(0, traj.waypoints.length - 1);

      Offset catmullRomPoint = _calculateCatmullRom(
        _getAbsoluteOffset(traj.waypoints[p0].position, traj),
        _getAbsoluteOffset(traj.waypoints[p1].position, traj),
        _getAbsoluteOffset(traj.waypoints[p2].position, traj),
        _getAbsoluteOffset(traj.waypoints[p3].position, traj),
        localT
      );
      
      path.lineTo(catmullRomPoint.dx, catmullRomPoint.dy);
    }

    return path;
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

  Offset _getAbsoluteOffset(Offset pos, TrajectoryModel traj) {
    if (traj.targetRoomZoneId != null) {
      final room = rooms.where((r) => r.id == traj.targetRoomZoneId).firstOrNull;
      if (room != null) {
        // Relative coordinates (0..1) -> Absolute canvas coordinates (px)
        return Offset(room.x + pos.dx * room.width, room.y + pos.dy * room.height);
      }
    }
    // Absolute coordinates (m) -> Absolute canvas coordinates (px)
    return pos * scaleMeterToPixel;
  }

  @override
  bool shouldRepaint(covariant TrajectoryLayerPainter oldDelegate) {
    return true;
  }
}
