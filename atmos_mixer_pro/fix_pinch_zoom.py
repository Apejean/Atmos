import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# Find GestureDetector
content = content.replace("onPanUpdate: (details) {", "onScaleUpdate: (details) {\n              if (details.scale != 1.0) {\n                setState(() {\n                  _zoom = (_zoom * details.scale).clamp(0.2, 3.0);\n                });\n              }\n              setState(() {\n                _yaw -= details.focalPointDelta.dx * 0.005;\n                _pitch += details.focalPointDelta.dy * 0.005;\n                _pitch = _pitch.clamp(-math.pi / 2, math.pi / 2);\n              });\n            },\n            onPanUpdate: (details) {")

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)

