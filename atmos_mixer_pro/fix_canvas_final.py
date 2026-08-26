import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# remove the duplicates
content = content.replace("  bool _isMeasuringScale = false;\n  Offset? _measureStart;\n  Offset? _measureEnd;\n\n  bool _isSidebarOpen = false;\n\n", "")
# Wait, let's just use regex to remove any duplicate declarations

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
