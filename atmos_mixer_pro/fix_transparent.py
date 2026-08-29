import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# According to flutter webview docs on macos, `setBackgroundColor(Colors.transparent)` is known to cause `UnimplementedError: opaque is not implemented on macOS`.
# Let's check if the widget itself has transparent set in `WebViewWidget`.
old_web = """              child: WebViewWidget(
                controller: _webViewController!,
              ),"""
new_web = """              child: Container(
                color: const Color(0xFF0B0F14),
                child: WebViewWidget(
                  controller: _webViewController!,
                ),
              ),"""

content = content.replace(old_web, new_web)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
