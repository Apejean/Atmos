import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Add iso projection helper at the bottom
iso_helper = """
class IsoProjector {
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
}
"""
content += iso_helper

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
