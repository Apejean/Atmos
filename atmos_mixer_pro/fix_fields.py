import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

fields = """
  bool _isMeasuringScale = false;
  Offset? _measureStart;
  Offset? _measureEnd;
  bool _isSidebarOpen = false;
"""
content = content.replace("bool _isRoomInteracting = false;", "bool _isRoomInteracting = false;\n" + fields)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
