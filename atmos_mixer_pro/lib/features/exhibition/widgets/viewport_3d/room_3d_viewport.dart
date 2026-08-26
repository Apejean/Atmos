import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'dart:math' as math;
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';

class Room3DViewport extends ConsumerStatefulWidget {
  final bool showHeatmap;
  final double targetSPL;

  const Room3DViewport({
    super.key,
    required this.showHeatmap,
    required this.targetSPL,
  });

  @override
  ConsumerState<Room3DViewport> createState() => _Room3DViewportState();
}

class _Room3DViewportState extends ConsumerState<Room3DViewport> {
  double _orbitAngleX = math.pi / 4;
  double _orbitAngleY = math.pi / 6;
  final _isoViewController = TransformationController();

  @override
  void dispose() {
    _isoViewController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final blueprint = ref.watch(blueprintProvider);
    final speakers = ref.watch(speakerLayoutProvider);
    const lineCyan = Color(0xFF00B0FF);

    const scale = 50.0; // 50 pixels per meter in 3D view

    return GestureDetector(
      onPanUpdate: (details) {
        setState(() {
          _orbitAngleX += details.delta.dx * 0.01;
          _orbitAngleY -= details.delta.dy * 0.01;
          _orbitAngleY = _orbitAngleY.clamp(-math.pi / 2, math.pi / 2);
        });
      },
      child: Stack(
        children: [
          InteractiveViewer(
            transformationController: _isoViewController,
            boundaryMargin: const EdgeInsets.all(double.infinity),
            minScale: 0.1,
            maxScale: 10.0,
            constrained: false,
            child: SizedBox(
              width: 1200,
              height: 800,
              child: Stack(
                children: [
                  CustomPaint(
                    size: const Size(1200, 800),
                    painter: _IsoRoomPainter(
                      lineCyan: lineCyan,
                      blueprint: blueprint,
                      scale: scale,
                      orbitAngleX: _orbitAngleX,
                      orbitAngleY: _orbitAngleY,
                    ),
                  ),
                  if (widget.showHeatmap)
                    CustomPaint(
                      size: const Size(1200, 800),
                      painter: _ThermalHeatmapPainter(
                        speakers: speakers,
                        blueprint: blueprint,
                        scale: scale,
                        orbitAngleX: _orbitAngleX,
                        orbitAngleY: _orbitAngleY,
                        targetSPL: widget.targetSPL,
                      ),
                    ),
                  ...speakers.map((node) {
                    final proj = _IsoProjector(
                      scale: scale,
                      cx: 600,
                      cy: 400,
                      roomW: blueprint.canvasWidthMeters,
                      roomD: blueprint.canvasHeightMeters,
                      angleX: _orbitAngleX,
                      angleY: _orbitAngleY,
                    ).project(
                      node.x / (blueprint.canvasWidthMeters > 0 ? 1000.0 / blueprint.canvasWidthMeters : 1.0),
                      node.y / (blueprint.canvasHeightMeters > 0 ? 1000.0 / blueprint.canvasHeightMeters : 1.0),
                      node.heightZ,
                    );

                    return Positioned(
                      left: proj.dx - 16,
                      top: proj.dy - 16,
                      child: Container(
                        width: 32,
                        height: 32,
                        decoration: BoxDecoration(
                          color: const Color(0xFF141D2B),
                          shape: BoxShape.circle,
                          border: Border.all(color: const Color(0xFF00B0FF), width: 1.2),
                        ),
                        child: Center(
                          child: Text(
                            'S0${node.channel + 1}',
                            style: const TextStyle(
                              color: Color(0xFF38BDF8),
                              fontSize: 9,
                              fontWeight: FontWeight.bold,
                              fontFamily: 'monospace',
                            ),
                          ),
                        ),
                      ),
                    );
                  }),
                ],
              ),
            ),
          ),

          // Camera info overlay
          Positioned(
            bottom: 16,
            right: 16,
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
              decoration: BoxDecoration(
                color: const Color(0xFF111823).withValues(alpha: 0.8),
                borderRadius: BorderRadius.circular(4),
                border: Border.all(color: const Color(0xFF2A3D54), width: 0.8),
              ),
              child: Text(
                'Orbit: X ${(_orbitAngleX * 180 / math.pi).toStringAsFixed(0)}° / Y ${(_orbitAngleY * 180 / math.pi).toStringAsFixed(0)}°',
                style: const TextStyle(color: Color(0xFF94A3B8), fontSize: 10, fontFamily: 'monospace'),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _IsoProjector {
  final double scale;
  final double cx;
  final double cy;
  final double roomW;
  final double roomD;
  final double angleX;
  final double angleY;

  _IsoProjector({
    required this.scale,
    required this.cx,
    required this.cy,
    required this.roomW,
    required this.roomD,
    required this.angleX,
    required this.angleY,
  });

  Offset project(double px, double py, double pz) {
    // Translate to room center
    double tx = (px - roomW / 2) * scale;
    double ty = (py - roomD / 2) * scale;
    double tz = pz * scale;

    // Rotate around Z axis (Orbit X)
    double rx = tx * math.cos(angleX) - ty * math.sin(angleX);
    double ry = tx * math.sin(angleX) + ty * math.cos(angleX);

    // Rotate around X axis (Orbit Y) for perspective
    double finalY = ry * math.cos(angleY) - tz * math.sin(angleY);

    return Offset(cx + rx, cy + finalY);
  }
}

class _IsoRoomPainter extends CustomPainter {
  final Color lineCyan;
  final BlueprintData blueprint;
  final double scale;
  final double orbitAngleX;
  final double orbitAngleY;

  _IsoRoomPainter({
    required this.lineCyan,
    required this.blueprint,
    required this.scale,
    required this.orbitAngleX,
    required this.orbitAngleY,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final w = blueprint.canvasWidthMeters;
    final d = blueprint.canvasHeightMeters;
    final h = blueprint.ceilingHeightMeters;

    final projector = _IsoProjector(
      scale: scale,
      cx: size.width / 2,
      cy: size.height / 2,
      roomW: w,
      roomD: d,
      angleX: orbitAngleX,
      angleY: orbitAngleY,
    );

    // Draw floor grid (1m x 1m)
    final gridPaint = Paint()
      ..color = const Color(0xFF1E2A3A)
      ..strokeWidth = 0.8;

    for (double x = 0; x <= w + 0.01; x += 1.0) {
      final p1 = projector.project(x, 0, 0);
      final p2 = projector.project(x, d, 0);
      canvas.drawLine(p1, p2, gridPaint);
    }
    for (double y = 0; y <= d + 0.01; y += 1.0) {
      final p1 = projector.project(0, y, 0);
      final p2 = projector.project(w, y, 0);
      canvas.drawLine(p1, p2, gridPaint);
    }

    // Draw 3D Wireframe Box Edges
    final edgePaint = Paint()
      ..color = const Color(0xFF2A3D54)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.2;

    final corners = [
      projector.project(0, 0, 0),
      projector.project(w, 0, 0),
      projector.project(w, d, 0),
      projector.project(0, d, 0),
      projector.project(0, 0, h),
      projector.project(w, 0, h),
      projector.project(w, d, h),
      projector.project(0, d, h),
    ];

    // Bottom
    for (int i = 0; i < 4; i++) {
      canvas.drawLine(corners[i], corners[(i + 1) % 4], edgePaint);
    }
    // Top
    for (int i = 4; i < 8; i++) {
      canvas.drawLine(corners[i], corners[4 + (i + 1) % 4], edgePaint);
    }
    // Vertical
    for (int i = 0; i < 4; i++) {
      canvas.drawLine(corners[i], corners[i + 4], edgePaint);
    }

    // Draw Central Listener Dummy Head (Ear Level)
    final headPos = projector.project(w / 2, d / 2, blueprint.earLevelMeters);
    canvas.drawCircle(headPos, 14, Paint()..color = const Color(0xFF0E1A2B));
    canvas.drawCircle(
      headPos,
      14,
      Paint()
        ..color = const Color(0xFF00B0FF)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1.5,
    );
  }

  @override
  bool shouldRepaint(covariant _IsoRoomPainter oldDelegate) => true;
}

class _ThermalHeatmapPainter extends CustomPainter {
  final List<SpeakerNode> speakers;
  final BlueprintData blueprint;
  final double scale;
  final double orbitAngleX;
  final double orbitAngleY;
  final double targetSPL;

  _ThermalHeatmapPainter({
    required this.speakers,
    required this.blueprint,
    required this.scale,
    required this.orbitAngleX,
    required this.orbitAngleY,
    required this.targetSPL,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final projector = _IsoProjector(
      scale: scale,
      cx: size.width / 2,
      cy: size.height / 2,
      roomW: blueprint.canvasWidthMeters,
      roomD: blueprint.canvasHeightMeters,
      angleX: orbitAngleX,
      angleY: orbitAngleY,
    );

    for (final node in speakers) {
      final coverageM = node.dispersionDistance > 0 ? node.dispersionDistance / 100.0 : 4.0;
      final rx = (coverageM * scale) * math.sin(node.dispersionAngle / 2 * math.pi / 180);
      final tiltRad = node.pitchTilt * math.pi / 180;
      final tiltClampedRad = math.max(node.pitchTilt, 15.0) * math.pi / 180;

      final ry = rx * (1.0 + math.cos(tiltRad) / math.sin(tiltClampedRad));
      final dOffset = node.heightZ * (math.cos(tiltClampedRad) / math.sin(tiltClampedRad)) * scale;

      // Project center to floor (Z=0) considering rotation angle
      final panRad = node.rotation * math.pi / 180;
      final nodeNormX = (node.x / 1000.0) * blueprint.canvasWidthMeters;
      final nodeNormY = (node.y / 1000.0) * blueprint.canvasHeightMeters;

      final projectedCx = nodeNormX + (dOffset / scale) * math.sin(panRad);
      final projectedCy = nodeNormY - (dOffset / scale) * math.cos(panRad);

      final floorCenter = projector.project(projectedCx, projectedCy, 0);

      // Radial thermal gradient (Red -> Yellow -> Green -> Blue -> Transparent)
      final rect = Rect.fromCenter(center: floorCenter, width: rx * 2, height: ry * 2);

      final paint = Paint()
        ..shader = RadialGradient(
          colors: const [
            Color(0xCCFF1744), // Red
            Color(0xAAFFD600), // Yellow
            Color(0x8800E676), // Green
            Color(0x5500E5FF), // Blue
            Color(0x00000000), // Transparent
          ],
          stops: const [0.0, 0.3, 0.6, 0.85, 1.0],
        ).createShader(rect)
        ..blendMode = BlendMode.screen;

      // Draw ellipse squashed by isometric perspective
      canvas.save();
      canvas.translate(floorCenter.dx, floorCenter.dy);
      canvas.rotate(orbitAngleX);
      canvas.scale(1.0, math.sin(orbitAngleY).abs().clamp(0.2, 1.0));
      canvas.drawOval(Rect.fromCenter(center: Offset.zero, width: rx * 2, height: ry * 2), paint);
      canvas.restore();
    }
  }

  @override
  bool shouldRepaint(covariant _ThermalHeatmapPainter oldDelegate) => true;
}

