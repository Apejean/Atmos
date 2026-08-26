import re

path = "lib/features/exhibition/screens/speaker_canvas_screen.dart"
with open(path, "r") as f:
    content = f.read()

# Replace _HeatmapPainter to be more realistic (Red -> Yellow -> Green -> Blue) based on DBAP
# I'll just rewrite the whole _HeatmapPainter class.

painter_start = "class _HeatmapPainter extends CustomPainter {"
# Need to find where _HeatmapPainter ends. Usually before `class _SpeakerCanvasScreenState` or similar, but it's at the end of the file.
# The next class is usually nothing or maybe some other Painter. Let's just find `bool shouldRepaint(covariant _HeatmapPainter oldDelegate) => true;\n}`

painter_regex = r"class _HeatmapPainter extends CustomPainter \{[\s\S]*?bool shouldRepaint\(covariant _HeatmapPainter oldDelegate\) => true;\s*\n\}"

new_painter = """class _HeatmapPainter extends CustomPainter {
  final List<SpeakerNode> speakers;
  final double scale;

  _HeatmapPainter({required this.speakers, required this.scale});

  @override
  void paint(Canvas canvas, Size size) {
    if (speakers.isEmpty) return;

    for (var spk in speakers) {
      final center = Offset(spk.positionX * scale, spk.positionY * scale);
      
      // Calculate radius based on Z-height and tilt.
      // A higher speaker or tilted up speaker throws sound further.
      final baseRadius = 300.0;
      final zFactor = 1.0 + (spk.heightZ / 10.0);
      final tiltFactor = 1.0 - (spk.pitchTilt / 90.0).clamp(-0.5, 0.5);
      final radius = baseRadius * zFactor * tiltFactor;

      final rect = Rect.fromCircle(center: center, radius: radius);
      
      // L-Acoustics style Real Heatmap: Red(hot/close) -> Yellow -> Green -> Blue(cold/far)
      final gradient = RadialGradient(
        colors: [
          Colors.red.withOpacity(0.8),
          Colors.yellow.withOpacity(0.6),
          Colors.green.withOpacity(0.4),
          Colors.blue.withOpacity(0.2),
          Colors.transparent,
        ],
        stops: const [0.0, 0.2, 0.5, 0.8, 1.0],
      );

      final paint = Paint()
        ..shader = gradient.createShader(rect)
        ..blendMode = BlendMode.screen;

      // Apply rotation (Pan) to make it directional (like a real speaker coverage)
      canvas.save();
      canvas.translate(center.dx, center.dy);
      canvas.rotate(spk.rotation * math.pi / 180.0);
      canvas.translate(-center.dx, -center.dy);
      
      // Draw an oval to simulate the dispersion pattern (usually wider than deep depending on the horn)
      final dispersionRect = Rect.fromCenter(center: center, width: radius * 1.5, height: radius * 2.0);
      canvas.drawOval(dispersionRect, paint);
      
      canvas.restore();
    }
  }

  @override
  bool shouldRepaint(covariant _HeatmapPainter oldDelegate) => true;
}"""

content = re.sub(painter_regex, new_painter, content)

with open(path, "w") as f:
    f.write(content)
