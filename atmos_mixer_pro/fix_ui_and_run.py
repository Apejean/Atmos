import re

# Since the app failed to run properly and I got "opaque is not implemented on macOS",
# this is a known webview_flutter_wkwebview issue where `setBackgroundColor(Colors.transparent)` or `opaque` is used.
# Let's check `lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart` to fix this!

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

find_opaque = """      ..setJavaScriptMode(JavaScriptMode.unrestricted)
      ..setBackgroundColor(const Color(0xFF0B0F14))
      ..setOpaque(false)"""

replace_opaque = """      ..setJavaScriptMode(JavaScriptMode.unrestricted)
      ..setBackgroundColor(const Color(0xFF0B0F14))"""
      
# Wait, I don't know if setOpaque is there.
# Let's run a quick sed to remove it just in case.

import os
import subprocess
os.system("sed -i '' 's/..setOpaque(false)//g' lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart")
