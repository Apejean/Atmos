import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# WebViewController().setBackgroundColor(const Color(0x00000000))
# The error `opaque is not implemented on macOS` is usually related to `setBackgroundColor(Colors.transparent)`
# Or maybe the WebViewController itself has a bug on macos when setting opacity.

old_bg = """    final controller = WebViewController()
      ..setJavaScriptMode(JavaScriptMode.unrestricted)
      ..addJavaScriptChannel("""
new_bg = """    final controller = WebViewController()
      ..setJavaScriptMode(JavaScriptMode.unrestricted)
      // Note: do not use setBackgroundColor(Colors.transparent) on macOS webview_flutter, it causes opaque is not implemented on macOS
      ..addJavaScriptChannel("""
content = content.replace(old_bg, new_bg)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
