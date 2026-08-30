import re

with open('lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart', 'r') as f:
    content = f.read()

content = content.replace("speaker!", "speaker")

with open('lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart', 'w') as f:
    f.write(content)

with open('lib/features/exhibition/state/three_js_engine_provider.dart', 'r') as f:
    content = f.read()

# Just ignore curly braces info
