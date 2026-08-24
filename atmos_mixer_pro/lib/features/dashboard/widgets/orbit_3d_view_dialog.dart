import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';

/// 3D Orbit Perspective View Dialog / Widget for Speakers & 3D Objects
class Orbit3DViewDialog extends StatefulWidget {
  const Orbit3DViewDialog({super.key});

  @override
  State<Orbit3DViewDialog> createState() => _Orbit3DViewDialogState();
}

class _Orbit3DViewDialogState extends State<Orbit3DViewDialog> {
  double _yaw = 0.4;
  double _pitch = 0.5;
  double _zoom = 1.0;

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      backgroundColor: const Color(0xFF1E1E2C),
      title: Row(
        children: [
          const Icon(Icons.view_in_ar, color: AppColors.primaryNeon),
          const SizedBox(width: 8),
          const Text(
            '3D Orbit Perspective Viewer',
            style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold),
          ),
          const Spacer(),
          Text(
            'Yaw: ${(_yaw * 180 / math.pi).toInt()}° Pitch: ${(_pitch * 180 / math.pi).toInt()}°',
            style: const TextStyle(color: Colors.white54, fontSize: 11),
          ),
        ],
      ),
      content: SizedBox(
        width: 600,
        height: 420,
        child: GestureDetector(
          onPanUpdate: (details) {
            setState(() {
              _yaw += details.delta.dx * 0.01;
              _pitch = (_pitch + details.delta.dy * 0.01).clamp(-math.pi / 2, math.pi / 2);
            });
          },
          child: Container(
            decoration: BoxDecoration(
              color: const Color(0xFF101018),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: AppColors.primaryNeon.withValues(alpha: 0.3)),
            ),
            child: ClipRRect(
              borderRadius: BorderRadius.circular(8),
              child: CustomPaint(
                size: Size.infinite,
                painter: _Orbit3DPainter(yaw: _yaw, pitch: _pitch, zoom: _zoom),
              ),
            ),
          ),
        ),
      ),
      actions: [
        Row(
          children: [
            const Text('Zoom: ', style: TextStyle(color: Colors.white54, fontSize: 12)),
            IconButton(
              icon: const Icon(Icons.remove_circle_outline, color: Colors.white70, size: 20),
              onPressed: () => setState(() => _zoom = (_zoom - 0.1).clamp(0.5, 2.5)),
            ),
            IconButton(
              icon: const Icon(Icons.add_circle_outline, color: Colors.white70, size: 20),
              onPressed: () => setState(() => _zoom = (_zoom + 0.1).clamp(0.5, 2.5)),
            ),
            const Spacer(),
            ElevatedButton(
              style: ElevatedButton.styleFrom(
                backgroundColor: AppColors.primaryNeon,
                foregroundColor: Colors.black,
              ),
              onPressed: () => Navigator.of(context).pop(),
              child: const Text('Close'),
            ),
          ],
        ),
      ],
    );
  }
}

class _Orbit3DPainter extends CustomPainter {
  final double yaw;
  final double pitch;
  final double zoom;

  _Orbit3DPainter({required this.yaw, required this.pitch, required this.zoom});

  @override
  void paint(Canvas canvas, Size size) {
    final Offset center = Offset(size.width / 2, size.height / 2);
    final double radius = math.min(size.width, size.height) * 0.35 * zoom;

    // Draw 3D Room Bounding Grid
    final Paint gridPaint = Paint()
      ..color = Colors.white12
      ..strokeWidth = 1.0
      ..style = PaintingStyle.stroke;

    canvas.drawCircle(center, radius, gridPaint);
    canvas.drawOval(
      Rect.fromCenter(
        center: center,
        width: radius * 2,
        height: radius * math.sin(pitch).abs() * 2 + 10,
      ),
      gridPaint,
    );

    // Draw 24 Speaker Positions in 3D Space
    final Paint speakerPaint = Paint()
      ..color = Colors.cyanAccent
      ..style = PaintingStyle.fill;

    for (int i = 0; i < 24; i++) {
      final double angle = (i / 24.0) * math.pi * 2 + yaw;
      final double x = center.dx + radius * math.cos(angle);
      final double y = center.dy + radius * math.sin(angle) * math.sin(pitch);
      canvas.drawCircle(Offset(x, y), 5.0, speakerPaint);
    }

    // Draw 3D Sound Source Object Position
    final Paint objectPaint = Paint()
      ..color = Colors.amberAccent
      ..style = PaintingStyle.fill;

    final double objAngle = yaw + 0.8;
    final double objX = center.dx + (radius * 0.5) * math.cos(objAngle);
    final double objY = center.dy + (radius * 0.5) * math.sin(objAngle) * math.sin(pitch);

    canvas.drawCircle(Offset(objX, objY), 8.0, objectPaint);

    // Draw Object Line Pointer
    final Paint linePaint = Paint()
      ..color = Colors.amberAccent.withValues(alpha: 0.5)
      ..strokeWidth = 1.5;
    canvas.drawLine(center, Offset(objX, objY), linePaint);
  }

  @override
  bool shouldRepaint(covariant _Orbit3DPainter oldDelegate) =>
      oldDelegate.yaw != yaw || oldDelegate.pitch != pitch || oldDelegate.zoom != zoom;
}
