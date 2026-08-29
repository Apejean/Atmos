import re

content = """class _GridPainter extends CustomPainter {
  final double scale;

  _GridPainter(this.scale);

  @override
  void paint(Canvas canvas, Size size) {
    final double safeScale =
        (scale > 0 && !scale.isNaN && !scale.isInfinite) ? scale : 40.0;
    
    // Background fill
    canvas.drawRect(
      Rect.fromLTWH(0, 0, size.width, size.height), 
      Paint()..color = const Color(0xFF131B26)
    );

    final gridPaint = Paint()
      ..color = Colors.lightBlueAccent.withValues(alpha: 0.1)
      ..strokeWidth = 1.0;
      
    final majorGridPaint = Paint()
      ..color = Colors.lightBlueAccent.withValues(alpha: 0.2)
      ..strokeWidth = 1.5;

    for (double i = 0; i <= size.width; i += safeScale) {
      final isMajor = (i / safeScale).round() % 5 == 0;
      canvas.drawLine(Offset(i, 0), Offset(i, size.height), isMajor ? majorGridPaint : gridPaint);
    }
    for (double i = 0; i <= size.height; i += safeScale) {
      final isMajor = (i / safeScale).round() % 5 == 0;
      canvas.drawLine(Offset(0, i), Offset(size.width, i), isMajor ? majorGridPaint : gridPaint);
    }
    
    // Draw 3D Room Walls
    final wallColor = Colors.lightBlueAccent.withValues(alpha: 0.7);
    final wallPaint = Paint()
      ..color = wallColor
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2.0;
      
    final fillPaint = Paint()
      ..color = Colors.lightBlueAccent.withValues(alpha: 0.05)
      ..style = PaintingStyle.fill;
      
    final double depth = 40.0;
    final double margin = 60.0;
    
    final innerRect = Rect.fromLTRB(margin, margin, size.width - margin, size.height - margin);
    final outerRect = Rect.fromLTRB(margin - depth, margin - depth, size.width - margin + depth, size.height - margin + depth);

    // Fill walls
    final path = Path()
      ..addRect(outerRect)
      ..addRect(innerRect)
      ..fillType = PathFillType.evenOdd;
    canvas.drawPath(path, fillPaint);
      
    // Draw outlines
    canvas.drawRect(innerRect, wallPaint);
    canvas.drawRect(outerRect, wallPaint);
    
    // Connect corners
    canvas.drawLine(outerRect.topLeft, innerRect.topLeft, wallPaint);
    canvas.drawLine(outerRect.topRight, innerRect.topRight, wallPaint);
    canvas.drawLine(outerRect.bottomLeft, innerRect.bottomLeft, wallPaint);
    canvas.drawLine(outerRect.bottomRight, innerRect.bottomRight, wallPaint);
    
    // Draw door opening on right wall
    final doorStart = size.height / 2 - 40;
    final doorEnd = size.height / 2 + 40;
    canvas.drawLine(Offset(outerRect.right, doorStart), Offset(innerRect.right, doorStart), wallPaint);
    canvas.drawLine(Offset(outerRect.right, doorEnd), Offset(innerRect.right, doorEnd), wallPaint);
    
    // Dimension lines
    final dimPaint = Paint()..color = Colors.white54..strokeWidth = 1.0;
    // Top dimension
    canvas.drawLine(Offset(margin, margin - depth - 20), Offset(size.width - margin, margin - depth - 20), dimPaint);
    canvas.drawLine(Offset(margin, margin - depth - 25), Offset(margin, margin - depth - 15), dimPaint);
    canvas.drawLine(Offset(size.width - margin, margin - depth - 25), Offset(size.width - margin, margin - depth - 15), dimPaint);
    
    final textPainter = TextPainter(textDirection: TextDirection.ltr);
    final wMeters = ((size.width - 2*margin) / safeScale).toStringAsFixed(1);
    textPainter.text = TextSpan(text: wMeters, style: const TextStyle(color: Colors.white70, fontSize: 12, backgroundColor: Color(0xFF131B26)));
    textPainter.layout();
    textPainter.paint(canvas, Offset(size.width / 2 - textPainter.width / 2, margin - depth - 28));

    // Draw central user icon
    final center = Offset(size.width / 2, size.height / 2);
    canvas.drawCircle(center, 24, Paint()..color = Colors.lightBlueAccent.withValues(alpha: 0.2)..style=PaintingStyle.fill);
    canvas.drawCircle(center, 24, Paint()..color = Colors.lightBlueAccent..style=PaintingStyle.stroke..strokeWidth=2);
    // Draw a simple person shape inside
    canvas.drawCircle(Offset(center.dx, center.dy - 6), 8, Paint()..color = Colors.lightBlueAccent..style=PaintingStyle.fill);
    final bodyPath = Path()
      ..moveTo(center.dx - 14, center.dy + 16)
      ..quadraticBezierTo(center.dx, center.dy, center.dx + 14, center.dy + 16)
      ..close();
    canvas.drawPath(bodyPath, Paint()..color = Colors.lightBlueAccent..style=PaintingStyle.fill);
  }

  @override
  bool shouldRepaint(covariant _GridPainter oldDelegate) =>
      oldDelegate.scale != scale;
}
"""

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    file_content = f.read()
    
# regex replace the entire _GridPainter class
import re
pattern = r"class _GridPainter extends CustomPainter \{.*?\n\}\n"
file_content = re.sub(pattern, content + "\n", file_content, flags=re.DOTALL)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(file_content)

