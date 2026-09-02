import os
import re

# 1. Fix Speaker Inspector Panel
path1 = 'lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart'
with open(path1, 'r') as f:
    content1 = f.read()

import_line = "import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';"
if import_line not in content1:
    content1 = content1.replace("import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';", "import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';\n" + import_line)

with open(path1, 'w') as f:
    f.write(content1)

# 2. Fix tuning_modal.dart ChannelTuningParams
path2 = 'lib/features/settings/widgets/tuning_modal.dart'
with open(path2, 'r') as f:
    content2 = f.read()

content2 = re.sub(r'ChannelTuningParams\(\s*channel:', r'ChannelTuningParams(phaseInvert: false, gainDb: 0.0, channel:', content2)

with open(path2, 'w') as f:
    f.write(content2)

print("Compile errors fixed properly.")
