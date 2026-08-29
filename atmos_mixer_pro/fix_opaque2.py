import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# Try to force setBackgroundColor to a solid color to prevent opaque error.
old_bg = """      // Note: do not use setBackgroundColor(Colors.transparent) on macOS webview_flutter, it causes opaque is not implemented on macOS
      ..addJavaScriptChannel("""
new_bg = """      ..setBackgroundColor(const Color(0xFF0B0F14)) // Set explicitly to prevent opaque error
      ..addJavaScriptChannel("""
content = content.replace(old_bg, new_bg)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
