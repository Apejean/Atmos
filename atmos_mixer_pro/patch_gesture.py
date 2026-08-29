import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# I want to add double-tap zoom to WebViewWidget?
# Actually, the WebView intercepts all touches. If I wrap WebViewWidget in a GestureDetector, doubleTap might NOT trigger because WebView consumes pointers!
# The best way to handle double-click in WebViewController is directly inside HTML/JS, which the other agent already did (`window.addEventListener('dblclick', () => { ... })`).
# Wait, I just modified the JS `dblclick` listener to respect `window.currentViewName`.
# Is that working?
