import os
import re

def fix_file(file_path):
    with open(file_path, 'r') as f:
        content = f.read()

    # Add import
    if 'room_zone_state.dart' not in content:
        content = content.replace(
            "import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';",
            "import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';\nimport 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';"
        )
    
    # Replace blueprint.rooms with ref.watch(roomZoneProvider)
    # But wait, in the Painter it receives BlueprintData. We should also pass List<RoomZone> to painters.
    
    with open(file_path, 'w') as f:
        f.write(content)

fix_file('lib/features/exhibition/widgets/viewport_3d/room_3d_viewport.dart')
fix_file('lib/features/exhibition/widgets/hud/room_setup_window.dart')
