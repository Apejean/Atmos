import re

# Failed evaluating JavaScript?
# Which javascript failed?
# In `dynamic_3d_room.dart`, when the UI builds, `_syncSceneData` is called, which calls `_webViewController!.runJavaScript("window.updateScene(...)")`.
# If `window.updateScene` doesn't exist yet (because the page hasn't finished loading), it will throw!
# The other agent probably broke this by changing the Three.js HTML without handling load state properly.
