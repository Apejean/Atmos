import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# remove the block I added
state_vars = """  bool _isMeasuringScale = false;
  Offset? _measureStart;
  Offset? _measureEnd;
  bool _isSidebarOpen = false;
"""
content = content.replace("bool _isRoomInteracting = false;\n" + state_vars, "bool _isRoomInteracting = false;")

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
