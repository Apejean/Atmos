import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Add import
if 'room_setup_window.dart' not in content:
    content = content.replace(
        "import 'package:flutter/material.dart';",
        "import 'package:flutter/material.dart';\nimport 'package:atmos_mixer_pro/features/exhibition/widgets/hud/room_setup_window.dart';"
    )

# Find the Stack in build method and insert the RoomSetupWindow floating on bottom-left
# Let's see the current build method structure. The easiest way is to wrap the Scaffold body in a Stack.

body_start = content.find('body: ')
if body_start != -1:
    # Basic structural replace
    # We will replace `body: Column(` with `body: Stack(children: [ Column(`, 
    # and add `Positioned(left: 16, bottom: 16, child: RoomSetupWindow())` at the end of the Stack
    
    # Actually, simpler: replace `body: InteractiveViewer` or whatever is the root.
    pass

# Let's inspect the file first
