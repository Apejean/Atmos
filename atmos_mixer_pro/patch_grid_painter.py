import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

replacement = """class _SciFiGridPainter extends CustomPainter {
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

content = re.sub(r'class _SciFiGridPainter extends CustomPainter \{[\s\S]*?bool shouldRepaint\(covariant _SciFiGridPainter oldDelegate\) => true;\n\}', replacement, content)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
