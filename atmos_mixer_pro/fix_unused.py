import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

content = re.sub(r'  void _handlePointerSignal\(PointerSignalEvent event\) \{.*?\n  \}\n', '', content, flags=re.DOTALL)
content = re.sub(r'  void _handleScaleStart\(ScaleStartDetails details\) \{.*?\n  \}\n', '', content, flags=re.DOTALL)
content = re.sub(r'  void _handleScaleUpdate\(ScaleUpdateDetails details\) \{.*?\n    \}\n  \}\n', '', content, flags=re.DOTALL)
content = re.sub(r'  void _resetCamera\(\) \{.*?\n  \}\n', '', content, flags=re.DOTALL)
content = content.replace("double _cameraDistance = 6.5;\n  double _basePinchDistance = 6.5;\n  double _yaw = 45.0;\n  double _pitch = 65.0;\n", "")

# Remove orbitString
content = re.sub(r"    final orbitString = '\$\{_yaw\.toStringAsFixed\(0\)\}deg \$\{_pitch\.toStringAsFixed\(0\)\}deg \$\{_cameraDistance\.toStringAsFixed\(1\)\}m';\n", "", content)

# Remove unused PointerSignalEvent import if it exists
content = content.replace("import 'package:flutter/gestures.dart';\n", "")

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
