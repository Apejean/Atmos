import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

target1 = """                  child: RepaintBoundary(
                    child: SizedBox(
                      width: _getCanvasWidth(ref),
                      height: _getCanvasHeight(ref),"""

replacement1 = """                  child: RepaintBoundary(
                    child: Transform(
                      alignment: Alignment.center,
                      transform: Matrix4.identity()
                        ..setEntry(3, 2, 0.001)
                        ..rotateX(0.7),
                      child: SizedBox(
                        width: _getCanvasWidth(ref),
                        height: _getCanvasHeight(ref),"""

target2 = """                          ],
                        ),
                      ),
                    ),
                  ),
                if (_isSidebarOpen)"""

replacement2 = """                          ],
                        ),
                      ),
                    ),
                  ),
                  ),
                if (_isSidebarOpen)"""

content = content.replace(target1, replacement1)
content = content.replace(target2, replacement2)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)

