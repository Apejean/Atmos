import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# According to flutter documentation, webview_flutter_wkwebview sets opaque: false by default when setBackgroundColor is called, which causes `opaque is not implemented on macOS` error.
# The workaround is to clear the background color explicitly, or ensure we don't set it. Wait, the code doesn't even set background color? Let's check `..setBackgroundColor` again.
# Wait, maybe there's a setBackgroundColor(Colors.transparent) that we missed? Let's check.
