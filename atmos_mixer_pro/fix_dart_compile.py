import os

# Fix 1: Add import for blueprint_state.dart
path = 'lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart'
with open(path, 'r') as f:
    content = f.read()

import_line = "import '../../state/blueprint_state.dart';"
if import_line not in content:
    content = content.replace("import '../../state/speaker_layout_state.dart';", "import '../../state/speaker_layout_state.dart';\n" + import_line)

with open(path, 'w') as f:
    f.write(content)

# Fix 2: 'phaseInvert' missing in tuning_modal.dart
path2 = 'lib/features/settings/widgets/tuning_modal.dart'
with open(path2, 'r') as f:
    content2 = f.read()

content2 = content2.replace("ChannelTuningParams(", "ChannelTuningParams(phaseInvert: false, ")

with open(path2, 'w') as f:
    f.write(content2)

print("Dart compile errors fixed.")
