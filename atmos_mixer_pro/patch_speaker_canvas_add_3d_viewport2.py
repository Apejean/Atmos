import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Add import for Room3DViewport
if 'Room3DViewport' not in content:
    content = content.replace(
        "import 'package:atmos_mixer_pro/features/exhibition/widgets/hud/speaker_inspector_panel.dart';",
        "import 'package:atmos_mixer_pro/features/exhibition/widgets/hud/speaker_inspector_panel.dart';\nimport 'package:atmos_mixer_pro/features/exhibition/widgets/viewport_3d/room_3d_viewport.dart';"
    )

# Split the body into 2D Top View and 3D Viewport
# Replace:
#             Positioned.fill(
#               child: LayoutBuilder(
#           builder: (context, constraints) {
#             _viewportSize = Size(constraints.maxWidth, constraints.maxHeight);
#             return Stack( ... )

# The problem is replacing this cleanly. Let's find `body: Stack(`
# Inside the Stack, the first child is `Positioned.fill( child: LayoutBuilder( ... ) )`
# We want to replace this `Positioned.fill` with a `Column` that splits the space.

# First, find the `body: Stack(`
body_start = content.find("body: Stack(")
positioned_fill = content.find("Positioned.fill(", body_start)

# It's better to just wrap the original `LayoutBuilder` in an `Expanded` inside a `Column`.
# Wait, `Positioned.fill` implies it's inside a `Stack`.
# We can change `Positioned.fill( child: LayoutBuilder(...) )`
# to:
# Positioned.fill(
#   child: Column(
#     children: [
#       Expanded(flex: 3, child: LayoutBuilder(...)),
#       Container(height: 2, color: Colors.blueAccent),
#       const Expanded(flex: 2, child: Room3DViewport()),
#     ]
#   )
# )

import os
os.system('cat lib/features/exhibition/screens/speaker_canvas_screen.dart | grep -n "Positioned.fill("')

