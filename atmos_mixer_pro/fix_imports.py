import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

imports = """
import 'package:atmos_mixer_pro/features/exhibition/widgets/trajectory_layer_painter.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/trajectory_sidebar_widget.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/trajectory_editor_toolbar.dart';
"""

# Insert imports after standard imports
content = content.replace("import 'package:flutter/material.dart';", "import 'package:flutter/material.dart';\n" + imports)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
