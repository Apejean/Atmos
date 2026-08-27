import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:model_viewer_plus/model_viewer_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/viewport_3d/speaker_3d_box.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';

class Dynamic3DRoom extends ConsumerStatefulWidget {
  final Function(String)? onSpeakerTapped;
  const Dynamic3DRoom({super.key, this.onSpeakerTapped});

  @override
  ConsumerState<Dynamic3DRoom> createState() => _Dynamic3DRoomState();
}

class _Dynamic3DRoomState extends ConsumerState<Dynamic3DRoom> {
  double angleX = 0.0;
  double angleY = 0.0;
  
  @override
  Widget build(BuildContext context) {
    final speakers = ref.watch(speakerLayoutProvider);
    final bp = ref.watch(blueprintProvider);

    return Scaffold(
      backgroundColor: const Color(0xFF0F111A), // Dark canvas background
      body: Stack(
        children: [
          // Background Painter (Floor grid and back walls)
          CustomPaint(
            size: Size.infinite,
            painter: DynamicRoomPainter(
              roomW: bp.canvasWidthMeters,
              roomD: bp.canvasHeightMeters,
              roomH: 3.5, // Standard height
              angleX: angleX,
              angleY: angleY,
              drawBackground: true,
            ),
          ),
          
          // 3D Head Model
          IgnorePointer(
            child: ModelViewer(
              src: 'assets/models/listener_head.glb',
              alt: '3D Listener Head',
              autoRotate: false,
              cameraControls: false,
              disableZoom: true,
              disablePan: true,
              cameraOrbit: '\${angleY * 180 / math.pi}deg \${90 + angleX * 180 / math.pi}deg 105%',
              interactionPrompt: InteractionPrompt.none,
            ),
          ),
          
          // Speakers
          ...speakers.map((spk) {
            return Positioned(
              left: 0, top: 0, right: 0, bottom: 0, // Fill stack to use CustomPainter coordinates or Align
              child: _buildSpeaker(spk, bp.canvasWidthMeters, bp.canvasHeightMeters),
            );
          }),

          // Foreground Painter (Front walls)
          CustomPaint(
            size: Size.infinite,
            painter: DynamicRoomPainter(
              roomW: bp.canvasWidthMeters,
              roomD: bp.canvasHeightMeters,
              roomH: 3.5,
              angleX: angleX,
              angleY: angleY,
              drawBackground: false,
            ),
          ),
          
          // Gesture overlay
          GestureDetector(
            onPanUpdate: (details) {
              setState(() {
                angleY += details.delta.dx * 0.01;
                angleX += details.delta.dy * 0.01;
                angleX = angleX.clamp(-math.pi / 2, math.pi / 2);
              });
            },
          ),
          
          // UI Overlays
          Positioned(
            bottom: 32,
            right: 32,
            child: FloatingActionButton.extended(
              onPressed: () {
                // Add speaker logic
                final newId = 'spk_\${DateTime.now().millisecondsSinceEpoch}';
                final newNode = SpeakerNode(
                  id: newId,
                  x: bp.canvasWidthMeters / 2,
                  y: bp.canvasHeightMeters / 2,
                  channel: speakers.length,
                );
                ref.read(speakerLayoutProvider.notifier).addSpeaker(newNode);
              },
              backgroundColor: Colors.lightBlueAccent,
              icon: const Icon(Icons.add),
              label: const Text('Add Speaker'),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildSpeaker(SpeakerNode spk, double w, double d) {
    // Project speaker to 2D
    final project = _project3D(
      spk.x - w / 2,
      spk.y - d / 2,
      spk.heightZ - 1.75, // Assuming 3.5m height
      angleX,
      angleY,
    );

    return LayoutBuilder(
      builder: (context, constraints) {
        final cx = constraints.maxWidth / 2;
        final cy = constraints.maxHeight / 2;
        
        return Positioned(
          left: cx + project.dx - 40, // 40 is half of scaled box width roughly
          top: cy + project.dy - 70, // roughly half of scaled box height
          child: GestureDetector(
            onTap: () {
              if (widget.onSpeakerTapped != null) {
                widget.onSpeakerTapped!(spk.id);
              }
            },
            child: Transform.scale(
              scale: 0.5,
              child: Speaker3DBox(
                angleX: angleX + spk.pitchTilt * math.pi / 180,
                angleY: angleY + spk.panDeg * math.pi / 180,
                angleZ: 0,
              ),
            ),
          ),
        );
      },
    );
  }

  Offset _project3D(double x, double y, double z, double ax, double ay) {
    // Rotate Y (Pan)
    final cosY = math.cos(ay);
    final sinY = math.sin(ay);
    final rx = x * cosY - y * sinY;
    final ry = x * sinY + y * cosY;

    // Rotate X (Tilt)
    final cosX = math.cos(ax);
    final sinX = math.sin(ax);
    final rz = ry * sinX + z * cosX;
    final ry2 = ry * cosX - z * sinX;

    final scale = 100.0; // pixels per meter roughly
    return Offset(rx * scale, -ry2 * scale);
  }
}

class DynamicRoomPainter extends CustomPainter {
  final double roomW;
  final double roomD;
  final double roomH;
  final double angleX;
  final double angleY;
  final bool drawBackground;

  DynamicRoomPainter({
    required this.roomW,
    required this.roomD,
    required this.roomH,
    required this.angleX,
    required this.angleY,
    required this.drawBackground,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final center = Offset(size.width / 2, size.height / 2);

    Offset project3D(double x, double y, double z) {
      x -= roomW / 2;
      y -= roomD / 2;
      z -= roomH / 2;

      final cosY = math.cos(angleY);
      final sinY = math.sin(angleY);
      final rx = x * cosY - y * sinY;
      final ry = x * sinY + y * cosY;

      final cosX = math.cos(angleX);
      final sinX = math.sin(angleX);
      final rz = ry * sinX + z * cosX;
      final ry2 = ry * cosX - z * sinX;

      final scale = 100.0;
      return Offset(center.dx + rx * scale, center.dy - ry2 * scale);
    }

    final paintLine = Paint()
      ..color = Colors.lightBlueAccent.withValues(alpha: 0.2)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.0;
      
    final paintSolidLine = Paint()
      ..color = Colors.lightBlueAccent.withValues(alpha: 0.6)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2.0;

    if (drawBackground) {
      // Draw Floor grid (4x4 or based on meters)
      for (double x = 0; x <= roomW; x += math.max(1.0, roomW / 4)) {
        canvas.drawLine(project3D(x, 0, 0), project3D(x, roomD, 0), paintLine);
      }
      for (double y = 0; y <= roomD; y += math.max(1.0, roomD / 4)) {
        canvas.drawLine(project3D(0, y, 0), project3D(roomW, y, 0), paintLine);
      }

      // Draw bottom rect
      final b1 = project3D(0, 0, 0);
      final b2 = project3D(roomW, 0, 0);
      final b3 = project3D(roomW, roomD, 0);
      final b4 = project3D(0, roomD, 0);
      canvas.drawPath(Path()..addPolygon([b1, b2, b3, b4, b1], true), paintSolidLine);
      
      // We could split the pillars into background and foreground by checking their Z depth in camera space,
      // but for simplicity, we just draw the back pillars here if needed, or all pillars.
      // A more robust approach uses dot product to check if a face is facing away from the camera.
    } else {
      // Draw Top rect
      final t1 = project3D(0, 0, roomH);
      final t2 = project3D(roomW, 0, roomH);
      final t3 = project3D(roomW, roomD, roomH);
      final t4 = project3D(0, roomD, roomH);
      canvas.drawPath(Path()..addPolygon([t1, t2, t3, t4, t1], true), paintSolidLine);
      
      // Draw pillars
      final b1 = project3D(0, 0, 0);
      final b2 = project3D(roomW, 0, 0);
      final b3 = project3D(roomW, roomD, 0);
      final b4 = project3D(0, roomD, 0);
      canvas.drawLine(b1, t1, paintSolidLine);
      canvas.drawLine(b2, t2, paintSolidLine);
      canvas.drawLine(b3, t3, paintSolidLine);
      canvas.drawLine(b4, t4, paintSolidLine);
    }
  }

  @override
  bool shouldRepaint(covariant DynamicRoomPainter oldDelegate) {
    return oldDelegate.angleX != angleX || oldDelegate.angleY != angleY;
  }
}
