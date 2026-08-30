import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# WebViewController is throwing the error deep inside its macOS implementation when initialized.
# According to flutter issues, webview_flutter_wkwebview requires the `WebViewWidget` to be in the tree
# or some specific initialization order, but the error happens specifically at `opaque is not implemented`.
# This is usually because `setOpaque` is called by `setBackgroundColor`. 
# Wait, let's look at `webview_flutter_wkwebview` source...
# Or maybe the webview_flutter plugin needs an update. `flutter pub upgrade webview_flutter`
