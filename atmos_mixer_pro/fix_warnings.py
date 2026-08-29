import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

content = content.replace("final double PPM = 100.0;", "final double ppm = 100.0;")
content = content.replace(" * PPM", " * ppm")
content = content.replace("..scale(_zoom, _zoom, _zoom)", "..scaleByDouble(_zoom)")

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

content = content.replace("import 'dart:async';\n", "")

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)

