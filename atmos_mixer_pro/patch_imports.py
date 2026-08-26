import os
import re

files_to_fix = [
    'lib/features/exhibition/screens/speaker_canvas_screen.dart',
    'lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart',
    'lib/features/exhibition/widgets/viewport_3d/room_3d_viewport.dart'
]

for file_path in files_to_fix:
    with open(file_path, 'r') as f:
        content = f.read()
    
    # Add imports to all these files
    if 'import \'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart\';' not in content:
        content = content.replace(
            "import 'package:flutter_riverpod/flutter_riverpod.dart';",
            "import 'package:flutter_riverpod/flutter_riverpod.dart';\nimport 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';"
        )
    
    with open(file_path, 'w') as f:
        f.write(content)
