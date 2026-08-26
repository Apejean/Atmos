import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Replace _SciFiGridPainter
target_grid = """class _SciFiGridPainter extends CustomPainter {
  final Color neonCyan;
  _SciFiGridPainter({required this.neonCyan});

  @override
  void paint(Canvas canvas, Size size) {
    // Draw Dark Grid
    final gridPaint = Paint()
      ..color = Colors.white.withValues(alpha: 0.05)
      ..strokeWidth = 1.0;
      
    final step = 50.0;
    for (double x = 0; x <= size.width; x += step) {
      canvas.drawLine(Offset(x, 0), Offset(x, size.height), gridPaint);
    }
    for (double y = 0; y <= size.height; y += step) {
      canvas.drawLine(Offset(0, y), Offset(size.width, y), gridPaint);
    }
    
    // Draw Outer Room Wireframe
    final roomPaint = Paint()
      ..color = Colors.white30
      ..strokeWidth = 2.0
      ..style = PaintingStyle.stroke;
    
    final roomRect = Rect.fromLTWH(100, 100, size.width - 200, size.height - 200);
    canvas.drawRect(roomRect, roomPaint);
    
    // Draw Corner Lines (Isometric depth illusion on 2D plane)
    canvas.drawLine(Offset(100, 100), Offset(130, 130), roomPaint);
    canvas.drawLine(Offset(size.width-100, 100), Offset(size.width-130, 130), roomPaint);
    canvas.drawLine(Offset(100, size.height-100), Offset(130, size.height-130), roomPaint);
    canvas.drawLine(Offset(size.width-100, size.height-100), Offset(size.width-130, size.height-130), roomPaint);
    
    final innerRect = Rect.fromLTWH(130, 130, size.width - 260, size.height - 260);
    canvas.drawRect(innerRect, roomPaint);
  }

  @override
  bool shouldRepaint(covariant _SciFiGridPainter oldDelegate) => true;
}"""

replacement_grid = """class _SciFiGridPainter extends CustomPainter {
  final Color neonCyan;
  final BlueprintData blueprint;

  _SciFiGridPainter({required this.neonCyan, required this.blueprint});

  @override
  void paint(Canvas canvas, Size size) {
    final projector = IsoProjector(
      scale: blueprint.scale,
      cx: size.width / 2,
      cy: size.height / 2 + 100, // Shift down slightly
      roomW: blueprint.canvasWidthMeters,
      roomD: blueprint.canvasHeightMeters,
    );

    final paintLine = Paint()
      ..color = Colors.white30
      ..strokeWidth = 1.0
      ..style = PaintingStyle.stroke;

    final w = blueprint.canvasWidthMeters;
    final d = blueprint.canvasHeightMeters;
    final h = blueprint.roomHeightMeters;

    // Floor Grid
    for (double x = 0; x <= w; x += 1.0) {
      canvas.drawLine(projector.project(x, 0, 0), projector.project(x, d, 0), paintLine);
    }
    for (double y = 0; y <= d; y += 1.0) {
      canvas.drawLine(projector.project(0, y, 0), projector.project(w, y, 0), paintLine);
    }

    // Walls
    final paintWall = Paint()
      ..color = neonCyan.withValues(alpha: 0.5)
      ..strokeWidth = 2.0
      ..style = PaintingStyle.stroke;

    final p000 = projector.project(0, 0, 0);
    final pW00 = projector.project(w, 0, 0);
    final p0D0 = projector.project(0, d, 0);
    final pWD0 = projector.project(w, d, 0);

    final p00H = projector.project(0, 0, h);
    final pW0H = projector.project(w, 0, h);
    final p0DH = projector.project(0, d, h);
    final pWDH = projector.project(w, d, h);

    // Verticals
    canvas.drawLine(p000, p00H, paintWall);
    canvas.drawLine(pW00, pW0H, paintWall);
    canvas.drawLine(p0D0, p0DH, paintWall);
    canvas.drawLine(pWD0, pWDH, paintWall);

    // Top Wireframe
    canvas.drawLine(p00H, pW0H, paintWall);
    canvas.drawLine(pW0H, pWDH, paintWall);
    canvas.drawLine(pWDH, p0DH, paintWall);
    canvas.drawLine(p0DH, p00H, paintWall);

    // Listener Dummy Head
    final centerEar = projector.project(w / 2, d / 2, blueprint.listeningHeightMeters);
    final headPaint = Paint()..color = neonCyan.withValues(alpha: 0.3)..style = PaintingStyle.fill;
    canvas.drawCircle(centerEar, 12.0, headPaint);
    final headOutline = Paint()..color = neonCyan..style = PaintingStyle.stroke..strokeWidth = 2.0;
    canvas.drawCircle(centerEar, 12.0, headOutline);
  }

  @override
  bool shouldRepaint(covariant _SciFiGridPainter oldDelegate) => true;
}"""

content = content.replace(target_grid, replacement_grid)

target_heatmap = """class _HeatmapPainter extends CustomPainter {
  final List<SpeakerNode> speakers;
  final double scale;
  final double earLevelZ = 1.2;

  _HeatmapPainter({required this.speakers, required this.scale});

  @override
  void paint(Canvas canvas, Size size) {
    if (speakers.isEmpty) return;

    for (var spk in speakers) {
      final double H = math.max(0.1, spk.heightZ - earLevelZ);
      final double tiltRad = spk.pitchTilt.clamp(1.0, 90.0) * math.pi / 180.0;
      final double halfDispersionRad = (spk.dispersionAngle / 2.0) * math.pi / 180.0;
      final double projectionOffset = (H / math.tan(tiltRad)) * scale;
      final double widthX = (H * math.tan(halfDispersionRad)) * scale;
      final double lengthY = (widthX / math.sin(tiltRad)); 

      final Gradient thermalGradient = RadialGradient(
        colors: [
          const Color(0xFFFF0000).withValues(alpha: 0.8), // Red (Core)
          const Color(0xFFFFFF00).withValues(alpha: 0.6), // Yellow
          const Color(0xFF00FF00).withValues(alpha: 0.4), // Green
          const Color(0xFF00FFFF).withValues(alpha: 0.2), // Cyan
          const Color(0x00000000),                        // Transparent
        ],
        stops: const [0.0, 0.2, 0.45, 0.75, 1.0],
      );

      final Paint heatPaint = Paint()
        ..blendMode = BlendMode.screen;

      final center = Offset(spk.x * scale, spk.y * scale);
      
      canvas.save();
      canvas.translate(center.dx, center.dy);
      canvas.rotate(spk.rotation * math.pi / 180.0);
      canvas.translate(0, -projectionOffset);
      canvas.scale(widthX / 100.0, lengthY / 100.0);

      final Rect rect = Rect.fromCircle(center: Offset.zero, radius: 100.0);
      heatPaint.shader = thermalGradient.createShader(rect);
      
      canvas.drawCircle(Offset.zero, 100.0, heatPaint);
      canvas.restore();
    }
  }

  @override
  bool shouldRepaint(covariant _HeatmapPainter oldDelegate) => true;
}"""

replacement_heatmap = """class _HeatmapPainter extends CustomPainter {
  final List<SpeakerNode> speakers;
  final BlueprintData blueprint;
  final double earLevelZ = 1.2;

  _HeatmapPainter({required this.speakers, required this.blueprint});

  @override
  void paint(Canvas canvas, Size size) {
    if (speakers.isEmpty) return;

    final projector = IsoProjector(
      scale: blueprint.scale,
      cx: size.width / 2,
      cy: size.height / 2 + 100,
      roomW: blueprint.canvasWidthMeters,
      roomD: blueprint.canvasHeightMeters,
    );

    // Apply the inverse of the isometric transform to the canvas, 
    // so we can just draw in the 2D floor plane!
    canvas.save();
    canvas.translate(projector.cx, projector.cy);
    canvas.scale(1.0, 0.5);
    canvas.rotate(math.pi / 4);

    // Now we are in a coordinate system where (0,0) is the center of the room.
    // The top-left corner of the room is at (-W/2 * scale, -D/2 * scale).
    final wScale = blueprint.canvasWidthMeters * blueprint.scale;
    final dScale = blueprint.canvasHeightMeters * blueprint.scale;
    canvas.translate(-wScale / 2, -dScale / 2);

    for (var spk in speakers) {
      final double H = math.max(0.1, spk.heightZ - earLevelZ);
      final double tiltRad = spk.pitchTilt.clamp(1.0, 90.0) * math.pi / 180.0;
      final double halfDispersionRad = (spk.dispersionAngle / 2.0) * math.pi / 180.0;
      final double projectionOffset = (H / math.tan(tiltRad)) * blueprint.scale;
      final double widthX = (H * math.tan(halfDispersionRad)) * blueprint.scale;
      final double lengthY = (widthX / math.sin(tiltRad)); 

      final Gradient thermalGradient = RadialGradient(
        colors: [
          const Color(0xFFFF0000).withValues(alpha: 0.8), // Red (Core)
          const Color(0xFFFFFF00).withValues(alpha: 0.6), // Yellow
          const Color(0xFF00FF00).withValues(alpha: 0.4), // Green
          const Color(0xFF00FFFF).withValues(alpha: 0.2), // Cyan
          const Color(0x00000000),                        // Transparent
        ],
        stops: const [0.0, 0.2, 0.45, 0.75, 1.0],
      );

      final Paint heatPaint = Paint()..blendMode = BlendMode.screen;

      final center = Offset(spk.x * blueprint.scale, spk.y * blueprint.scale);
      
      canvas.save();
      canvas.translate(center.dx, center.dy);
      // Rotation (Pan). 0 degrees means pointing right in 2D (which maps to isometric view angles)
      canvas.rotate(spk.rotation * math.pi / 180.0);
      canvas.translate(projectionOffset, 0); // Assuming 0 deg pan points along X axis
      canvas.scale(lengthY / 100.0, widthX / 100.0);

      final Rect rect = Rect.fromCircle(center: Offset.zero, radius: 100.0);
      heatPaint.shader = thermalGradient.createShader(rect);
      
      canvas.drawCircle(Offset.zero, 100.0, heatPaint);
      canvas.restore();
    }
    
    canvas.restore();
  }

  @override
  bool shouldRepaint(covariant _HeatmapPainter oldDelegate) => true;
}"""

content = content.replace(target_heatmap, replacement_heatmap)

# Fix IsoProjector definition
target_iso = """class IsoProjector {
  final double scale;
  final double offsetX;
  final double offsetY;
  
  IsoProjector({required this.scale, this.offsetX = 400, this.offsetY = 300});
  
  Offset project(double xMeters, double yMeters, double zMeters) {
    final double cos30 = math.cos(math.pi / 6);
    final double sin30 = math.sin(math.pi / 6);
    // x axis goes down-left, y axis goes down-right
    final double px = (yMeters - xMeters) * cos30 * scale;
    final double py = (xMeters + yMeters) * sin30 * scale - (zMeters * scale);
    return Offset(offsetX + px, offsetY + py);
  }
}"""

replacement_iso = """class IsoProjector {
  final double scale;
  final double cx;
  final double cy;
  final double roomW;
  final double roomD;
  
  IsoProjector({required this.scale, required this.cx, required this.cy, required this.roomW, required this.roomD});
  
  Offset project(double xMeters, double yMeters, double zMeters) {
    // Relative to center of room
    final double X = (xMeters - roomW / 2) * scale;
    final double Y = (yMeters - roomD / 2) * scale;
    final double Z = zMeters * scale;
    
    final double sx = cx + (X - Y) * 0.70710678;
    final double sy = cy + (X + Y) * 0.35355339 - Z;
    return Offset(sx, sy);
  }
}"""
content = content.replace(target_iso, replacement_iso)

# Update build to use blueprint
content = content.replace("_SciFiGridPainter(neonCyan: neonCyan)", "_SciFiGridPainter(neonCyan: neonCyan, blueprint: blueprint)")
content = content.replace("_HeatmapPainter(\n                                                speakers: nodes,\n                                                scale: blueprint.scale,", "_HeatmapPainter(\n                                                speakers: nodes,\n                                                blueprint: blueprint,")

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
