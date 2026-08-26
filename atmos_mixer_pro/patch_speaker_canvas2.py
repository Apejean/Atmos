import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Add import
if 'room_setup_window.dart' not in content:
    content = content.replace(
        "import 'package:flutter/material.dart';",
        "import 'package:flutter/material.dart';\nimport 'package:atmos_mixer_pro/features/exhibition/widgets/hud/room_setup_window.dart';"
    )

# Add a boolean state to toggle the window
if 'bool _isRoomSetupOpen = false;' not in content:
    content = content.replace(
        'bool _showHeatmap = false;',
        'bool _showHeatmap = false;\n  bool _isRoomSetupOpen = true;'
    )

# Wrap LayoutBuilder in a Stack
content = content.replace(
    'body: LayoutBuilder(',
    '''body: Stack(
          children: [
            Positioned.fill(
              child: LayoutBuilder('''
)

# Find the end of LayoutBuilder... it's hard with regex. Let's just find the end of the Scaffold.
# It ends with `bottomNavigationBar:` or just the end of the Scaffold widget.
content = content.replace(
    'floatingActionButton:',
    '''),
            if (_isRoomSetupOpen)
              Positioned(
                left: 16,
                bottom: 16,
                child: RoomSetupWindow(
                  onClose: () => setState(() => _isRoomSetupOpen = false),
                ),
              ),
          ],
        ),
        floatingActionButton:'''
)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
