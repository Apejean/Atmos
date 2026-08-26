import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# The errors mentioned:
# Undefined name '_measureStart', '_measureEnd', '_isSidebarOpen', 'TrajectorySidebarWidget', 'TrajectoryEditorToolbar'
# Missing 'TrajectoryModel', 'Waypoint' imports.

imports = """
import 'package:atmos_mixer_pro/features/exhibition/models/trajectory.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/trajectory_layer_painter.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/trajectory_sidebar_widget.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/trajectory_editor_toolbar.dart';
"""
if "models/trajectory.dart" not in content:
    content = content.replace("import 'package:flutter/material.dart';", "import 'package:flutter/material.dart';\n" + imports)


# Let's fix the undefined variables first. The script `patch_remove_traj_canvas.py` probably removed variable declarations but left usages.
# Let's check variables in class state
state_vars = """
  bool _isMeasuringScale = false;
  Offset? _measureStart;
  Offset? _measureEnd;
  bool _isSidebarOpen = false;
"""
# insert before _isRoomInteracting or similar
content = content.replace("bool _isRoomInteracting = false;", "bool _isRoomInteracting = false;\n" + state_vars)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
