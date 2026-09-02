import os

path = 'lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart'
with open(path, 'r') as f:
    content = f.read()

if "blueprint_state" not in content:
    content = content.replace("import 'package:atmos_mixer_pro/core/state/global_state.dart';", "import 'package:atmos_mixer_pro/core/state/global_state.dart';\nimport 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';")
    with open(path, 'w') as f:
        f.write(content)
    print("Added imports.")
else:
    print("Imports already exist.")
