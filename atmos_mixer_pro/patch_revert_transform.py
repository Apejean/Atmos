import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

target = """                  child: RepaintBoundary(
                    child: Transform(
                      alignment: Alignment.center,
                      transform: Matrix4.identity()
                        ..setEntry(3, 2, 0.001)
                        ..rotateX(0.7),
                      child: SizedBox(
                        width: _getCanvasWidth(ref),
                        height: _getCanvasHeight(ref),"""

replacement = """                  child: RepaintBoundary(
                    child: SizedBox(
                      width: _getCanvasWidth(ref),
                      height: _getCanvasHeight(ref),"""

content = content.replace(target, replacement)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)

