import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# WebViewController doesn't support setBackgroundColor(Colors.transparent) on macOS directly if we call setBackgroundColor with transparency? 
# Wait, `backgroundColor: const Color(0xFF0B0F14)` is fully opaque! It's not transparent.
# Let's check where `opaque is not implemented on macOS` is coming from.
# It comes from `.setBackgroundColor()` or something else?
# Actually, the error says: "UnimplementedError: opaque is not implemented on macOS".
# Let's check webview_flutter_wkwebview source or our code to find `opaque`.

find_background = """      ..setJavaScriptMode(JavaScriptMode.unrestricted)
      ..setBackgroundColor(const Color(0xFF0B0F14))"""
      
# Wait, `setBackgroundColor` calls `setOpaque(false)` under the hood sometimes?
# Let's remove `setBackgroundColor` completely from WebViewController just in case, because macOS might crash on it.

replace_background = """      ..setJavaScriptMode(JavaScriptMode.unrestricted)"""
content = content.replace(find_background, replace_background)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
