import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';

class Room3DViewport extends ConsumerStatefulWidget {
  const Room3DViewport({super.key});

  @override
  ConsumerState<Room3DViewport> createState() => _Room3DViewportState();
}

class _Room3DViewportState extends ConsumerState<Room3DViewport> {
  double _rotationX = 0.5; // Isometric down-tilt
  double _rotationY = 0.5; // Isometric pan

  @override
  Widget build(BuildContext context) {
    final layout = ref.watch(speakerLayoutProvider);
    final rooms = ref.watch(roomZoneProvider);
    
    final room = rooms.isNotEmpty ? rooms.first : null;
    if (room == null) return const Center(child: Text('No Room Configured', style: TextStyle(color: Colors.white54)));

    return GestureDetector(
      onPanUpdate: (details) {
        setState(() {
          _rotationY += details.delta.dx * 0.01;
          _rotationX += details.delta.dy * 0.01;
          // Clamp X rotation to prevent flipping upside down
          _rotationX = _rotationX.clamp(0.1, math.pi / 2 - 0.1);
        });
      },
      child: Container(
        color: const Color(0xFF1E2632), // Dark CAD background
        child: Stack(
          children: [
            // Center the 3D drawing
            Center(
              child: CustomPaint(
                size: const Size(600, 400),
                painter: _IsometricRoomPainter(
                  room: room,
                  speakers: layout,
                  rotationX: _rotationX,
                  rotationY: _rotationY,
                ),
              ),
            ),
            
            // HUD Overlay for 3D Viewport
            Positioned(
              top: 16,
              left: 16,
              child: const Text(
                '3D ISOMETRIC VIEWPORT (DRAG TO ROTATE)',
                style: TextStyle(color: Colors.lightBlueAccent, fontSize: 12, fontWeight: FontWeight.bold, letterSpacing: 1),
              ),
            ),
            Positioned(
              bottom: 16,
              right: 16,
              child: Row(
                children: [
                  IconButton(
                    icon: const Icon(Icons.refresh, color: Colors.white54, size: 20),
                    onPressed: () => setState(() {
                      _rotationX = 0.5;
                      _rotationY = 0.5;
                    }),
                    tooltip: 'Reset View',
                  ),
                ],
              ),
            )
          ],
        ),
      ),
    );
  }
}

class _IsometricRoomPainter extends CustomPainter {
  final RoomZone room;
  final List<SpeakerNode> speakers;
  final double rotationX;
  final double rotationY;

  _IsometricRoomPainter({
    required this.room,
    required this.speakers,
    required this.rotationX,
    required this.rotationY,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final center = Offset(size.width / 2, size.height / 2);
    final scale = math.min(size.width / (room.physicalWidth * 1.5), size.height / (room.physicalHeight * 1.5)) * 0.8;

    // Define 3D transformation matrices based on rotation
    Offset project3D(double x, double y, double z) {
      // Center origin
      x -= room.physicalWidth / 2;
      y -= room.physicalHeight / 2;
      z -= room.ceilingHeight / 2; // Center Z around middle of room

      // Rotate Y (Pan)
      final cosY = math.cos(rotationY);
      final sinY = math.sin(rotationY);
      final rx = x * cosY - y * sinY;
      final ry = x * sinY + y * cosY;

      // Rotate X (Tilt)
      final cosX = math.cos(rotationX);
      final sinX = math.sin(rotationX);
      final ry2 = ry * cosX - z * sinX;

      // Map to 2D
      return Offset(center.dx + rx * scale * 50, center.dy - ry2 * scale * 50); // Multiply by 50 to convert meters to rough pixels
    }

    final paintLine = Paint()
      ..color = Colors.lightBlueAccent.withValues(alpha: 0.3)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.0;

    final paintSolidLine = Paint()
      ..color = Colors.lightBlueAccent
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2.0;

    // 1. Draw Floor Grid (1m x 1m)
    final w = room.physicalWidth;
    final d = room.physicalHeight;
    final h = room.ceilingHeight;

    for (double x = 0; x <= w; x += 1.0) {
      canvas.drawLine(project3D(x, 0, 0), project3D(x, d, 0), paintLine);
    }
    for (double y = 0; y <= d; y += 1.0) {
      canvas.drawLine(project3D(0, y, 0), project3D(w, y, 0), paintLine);
    }

    // 2. Draw Room Wireframe (Cube)
    // Bottom rect
    final b1 = project3D(0, 0, 0);
    final b2 = project3D(w, 0, 0);
    final b3 = project3D(w, d, 0);
    final b4 = project3D(0, d, 0);
    canvas.drawPath(Path()..addPolygon([b1, b2, b3, b4, b1], true), paintSolidLine);

    // Top rect
    final t1 = project3D(0, 0, h);
    final t2 = project3D(w, 0, h);
    final t3 = project3D(w, d, h);
    final t4 = project3D(0, d, h);
    canvas.drawPath(Path()..addPolygon([t1, t2, t3, t4, t1], true), paintSolidLine);

    // Pillars
    canvas.drawLine(b1, t1, paintSolidLine);
    canvas.drawLine(b2, t2, paintSolidLine);
    canvas.drawLine(b3, t3, paintSolidLine);
    canvas.drawLine(b4, t4, paintSolidLine);

    // 3. Draw Listener (Center Head)
    final earLevel = room.earLevel;
    final headCenter = project3D(w/2, d/2, earLevel);
    
    final paintHead = Paint()
      ..color = Colors.amberAccent
      ..style = PaintingStyle.fill;
    canvas.drawCircle(headCenter, 6, paintHead);

    // 4. Draw Speakers
    for (final spk in speakers) {
      // Map 2D pixel coordinates (0-800) to meters
      // This is a rough estimation since the true ratio depends on the canvas zoom.
      // Assuming the room fills the 800x600 canvas for now.
      final meterX = (spk.x / 800) * w;
      final meterY = (spk.y / 600) * d;
      final meterZ = spk.heightZ;

      final spkPos = project3D(meterX, meterY, meterZ);
      
      final paintSpk = Paint()
        ..color = Colors.redAccent
        ..style = PaintingStyle.fill;
      
      // Draw speaker dot
      canvas.drawCircle(spkPos, 8, paintSpk);
      
      // Draw drop line to floor to show height
      final spkFloor = project3D(meterX, meterY, 0);
      canvas.drawLine(spkPos, spkFloor, Paint()..color = Colors.redAccent.withValues(alpha: 0.5)..strokeWidth=1..style=PaintingStyle.stroke);

      // Label
      final textPainter = TextPainter(
        text: TextSpan(
          text: 'Ch ${spk.channel + 1}',
          style: const TextStyle(color: Colors.white, fontSize: 10, fontWeight: FontWeight.bold),
        ),
        textDirection: TextDirection.ltr,
      );
      textPainter.layout();
      textPainter.paint(canvas, spkPos - const Offset(0, 20));
    }
  }

  @override
  bool shouldRepaint(covariant _IsometricRoomPainter oldDelegate) {
    return oldDelegate.room != room ||
        oldDelegate.speakers != speakers ||
        oldDelegate.rotationX != rotationX ||
        oldDelegate.rotationY != rotationY;
  }
}
