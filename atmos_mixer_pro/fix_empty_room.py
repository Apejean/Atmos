import subprocess
import re

# We already saved maker_room.dart earlier successfully!
with open('maker_room.dart', 'r') as f:
    content = f.read()

# Let's apply the exact visual styling requested by the user:
# Dark blue-grey background, simple thin blue-grey grid, NO fill for walls, just wireframe outlines.

new_painter = """class DynamicRoomPainter extends CustomPainter {
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

      final scale = 120.0; // Scale to fit better
      return Offset(center.dx + rx * scale, center.dy - ry2 * scale);
    }

    final paintGrid = Paint()
      ..color = const Color(0xFF5B6C86).withValues(alpha: 0.4) // Subtle blue-grey grid
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.0;
      
    final paintOutline = Paint()
      ..color = const Color(0xFF5B6C86).withValues(alpha: 0.8) // Stronger blue-grey for walls
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.5;

    if (drawBackground) {
      // Floor grid (4x4)
      for (int i = 1; i < 4; i++) {
        double x = roomW * (i / 4);
        canvas.drawLine(project3D(x, 0, 0), project3D(x, roomD, 0), paintGrid);
      }
      for (int i = 1; i < 4; i++) {
        double y = roomD * (i / 4);
        canvas.drawLine(project3D(0, y, 0), project3D(roomW, y, 0), paintGrid);
      }

      // Bottom rect
      final b1 = project3D(0, 0, 0);
      final b2 = project3D(roomW, 0, 0);
      final b3 = project3D(roomW, roomD, 0);
      final b4 = project3D(0, roomD, 0);
      canvas.drawPath(Path()..addPolygon([b1, b2, b3, b4, b1], true), paintOutline);
      
      // Pillars (wireframe, so draw all of them)
      final t1 = project3D(0, 0, roomH);
      final t2 = project3D(roomW, 0, roomH);
      final t3 = project3D(roomW, roomD, roomH);
      final t4 = project3D(0, roomD, roomH);
      
      canvas.drawLine(b1, t1, paintOutline);
      canvas.drawLine(b2, t2, paintOutline);
      canvas.drawLine(b3, t3, paintOutline);
      canvas.drawLine(b4, t4, paintOutline);
      
    } else {
      // Top rect
      final t1 = project3D(0, 0, roomH);
      final t2 = project3D(roomW, 0, roomH);
      final t3 = project3D(roomW, roomD, roomH);
      final t4 = project3D(0, roomD, roomH);
      canvas.drawPath(Path()..addPolygon([t1, t2, t3, t4, t1], true), paintOutline);
    }
  }

  @override
  bool shouldRepaint(covariant DynamicRoomPainter oldDelegate) {
    return oldDelegate.angleX != angleX || oldDelegate.angleY != angleY;
  }
}
"""
content = re.sub(r"class DynamicRoomPainter extends CustomPainter \{.*", new_painter, content, flags=re.DOTALL)

if "import 'package:model_viewer_plus/model_viewer_plus.dart';" not in content:
    content = content.replace("import 'package:flutter/material.dart';", "import 'package:flutter/material.dart';\nimport 'package:model_viewer_plus/model_viewer_plus.dart';")

content = content.replace("backgroundColor: const Color(0xFF0F111A)", "backgroundColor: const Color(0xFF131821)") # Match image background

# Fix _buildSpeaker to use Translate instead of Positioned to avoid ParentDataWidget error
build_speaker_pattern = r"  Widget _buildSpeaker\(SpeakerNode spk, double w, double d\) \{.*?  \}"
new_build_speaker = """  Widget _buildSpeaker(SpeakerNode spk, double w, double d) {
    final project = _project3D(
      spk.x - w / 2,
      spk.y - d / 2,
      spk.heightZ - 1.75, // Assuming 3.5m height
      angleX,
      angleY,
    );

    return Align(
      alignment: Alignment.center,
      child: Transform.translate(
        offset: Offset(project.dx, project.dy),
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
      ),
    );
  }"""
content = re.sub(build_speaker_pattern, new_build_speaker, content, flags=re.DOTALL)
content = content.replace("final scale = 100.0;", "final scale = 120.0;")

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)

