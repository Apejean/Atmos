import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/trajectory.dart';

class TrajectoryLayerPainter extends CustomPainter {
  final List<TrajectoryModel> trajectories;
  final String? focusedTrajectoryId;
  final double scaleMeterToPixel;

  TrajectoryLayerPainter({
    required this.trajectories,
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

      Offset currentPos = traj.getCurrentPositionMeter() * scaleMeterToPixel;
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
          Offset p = wp.position * scaleMeterToPixel;
          canvas.drawCircle(p, 6.0, waypointPaint);
          canvas.drawCircle(p, 6.0, waypointBorder);
        }
      }
    }
  }

  Path _buildSplinePath(TrajectoryModel traj) {
    final path = Path();
    if (traj.waypoints.isEmpty) return path;
    
    if (traj.waypoints.length == 1) {
      Offset p = traj.waypoints.first.position * scaleMeterToPixel;
      path.moveTo(p.dx, p.dy);
      path.lineTo(p.dx, p.dy);
      return path;
    }
    
    if (traj.waypoints.length == 2) {
      Offset p0 = traj.waypoints.first.position * scaleMeterToPixel;
      Offset p1 = traj.waypoints.last.position * scaleMeterToPixel;
      path.moveTo(p0.dx, p0.dy);
      path.lineTo(p1.dx, p1.dy);
      return path;
    }
    
    // For Catmull-Rom we sample points along the path
    Offset start = traj.waypoints.first.position * scaleMeterToPixel;
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
        Offset lastP = traj.waypoints.last.position * scaleMeterToPixel;
        path.lineTo(lastP.dx, lastP.dy);
        continue;
      }

      int p0 = (p1 - 1).clamp(0, traj.waypoints.length - 1);
      int p3 = (p2 + 1).clamp(0, traj.waypoints.length - 1);

      Offset catmullRomPoint = _calculateCatmullRom(
        traj.waypoints[p0].position * scaleMeterToPixel,
        traj.waypoints[p1].position * scaleMeterToPixel,
        traj.waypoints[p2].position * scaleMeterToPixel,
        traj.waypoints[p3].position * scaleMeterToPixel,
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

  @override
  bool shouldRepaint(covariant TrajectoryLayerPainter oldDelegate) {
    return oldDelegate.focusedTrajectoryId != focusedTrajectoryId ||
           oldDelegate.scaleMeterToPixel != scaleMeterToPixel;
  }
}
