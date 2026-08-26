import re

with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'r') as f:
    content = f.read()

if "import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';" not in content:
    content = content.replace("import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';", 
                              "import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';\nimport 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';")

with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'w') as f:
    f.write(content)
