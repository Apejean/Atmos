import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# Notice that `_syncSceneData` doesn't check `_isEngineReady` correctly or it calls it too early?
# Ah wait, `_syncSceneData` HAS:
# `if (_webViewController == null || !_isEngineReady) return;`
# So `_isEngineReady` MUST be true.
# Where is `_isEngineReady` set to true?

find_load = """      ..setNavigationDelegate(
        NavigationDelegate(
          onPageFinished: (String url) {
            if (mounted) {
              setState(() {
                _isEngineReady = true;
              });
              _syncSceneData();
            }
          },
        ),
      )"""
      
# Maybe `window.updateScene` is not defined in `studio_engine.html`?
