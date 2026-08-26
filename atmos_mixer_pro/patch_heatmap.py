import re
import os

path = "lib/features/exhibition/screens/speaker_canvas_screen.dart"
with open(path, "r") as f:
    content = f.read()

# Let's ensure the Heatmap looks exactly like the image: Green/Yellow/Red gradients.
# Right now it might just be drawing white arcs. We need it to be a real gradient.

old_paint = """      final Paint beamPaint = Paint()
        ..style = PaintingStyle.fill
        ..shader = ui.Gradient.radial(
          Offset.zero,
          axisX,
          [
            const Color(0xFF00FFCC).withOpacity(0.4),
            const Color(0xFF00FFCC).withOpacity(0.0),
          ],
          [0.0, 1.0],
        );"""

# The generated image shows Red near the speaker, then Yellow, then Green, then Blue far away.
new_paint = """      final Paint beamPaint = Paint()
        ..style = PaintingStyle.fill
        ..shader = ui.Gradient.radial(
          Offset.zero,
          axisX,
          [
            const Color(0xFFFF0000).withOpacity(0.7), // Red (Hot)
            const Color(0xFFFF8800).withOpacity(0.6), // Orange
            const Color(0xFFFFFF00).withOpacity(0.5), // Yellow
            const Color(0xFF00FF00).withOpacity(0.4), // Green (Optimal)
            const Color(0xFF0000FF).withOpacity(0.1), // Blue (Cold/Fade)
            const Color(0xFF0000FF).withOpacity(0.0), 
          ],
          [0.0, 0.2, 0.4, 0.6, 0.8, 1.0],
        );"""

if old_paint in content:
    content = content.replace(old_paint, new_paint)
else:
    # Use a regex if slight variations
    content = re.sub(r"final Paint beamPaint = Paint\(\)\s*\.\.style = PaintingStyle\.fill\s*\.\.shader = ui\.Gradient\.radial\([\s\S]*?\);", new_paint, content)

with open(path, "w") as f:
    f.write(content)

