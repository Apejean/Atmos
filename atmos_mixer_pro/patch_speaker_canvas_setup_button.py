import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Add a boolean state to toggle the window
if 'bool _isRoomSetupOpen = false;' not in content:
    content = content.replace(
        'bool _isBinauralEnabled = false;',
        'bool _isBinauralEnabled = false;\n  bool _isRoomSetupOpen = false;'
    )

if 'import \'package:atmos_mixer_pro/features/exhibition/widgets/hud/room_setup_window.dart\';' not in content:
    content = content.replace(
        "import 'package:flutter/material.dart';",
        "import 'package:flutter/material.dart';\nimport 'package:atmos_mixer_pro/features/exhibition/widgets/hud/room_setup_window.dart';"
    )

button_code = """              const SizedBox(width: 8),
              OutlinedButton.icon(
                icon: const Icon(Icons.picture_in_picture_alt_outlined, size: 16),
                label: const Text('ROOM SETUP'),
                style: OutlinedButton.styleFrom(
                  foregroundColor: _isRoomSetupOpen ? Colors.white : Colors.lightBlueAccent,
                  backgroundColor: _isRoomSetupOpen ? Colors.lightBlueAccent.withValues(alpha: 0.2) : Colors.transparent,
                  side: BorderSide(color: Colors.lightBlueAccent.withValues(alpha: _isRoomSetupOpen ? 0.8 : 0.4)),
                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 0),
                ),
                onPressed: () => setState(() => _isRoomSetupOpen = !_isRoomSetupOpen),
              ),"""

content = content.replace(
    '''              IconButton(
                tooltip: 'Set Blueprint',
                icon: const Icon(Icons.image, color: Colors.white70),
                onPressed: _pickBlueprint,
              ),''',
    button_code + '''\n              const SizedBox(width: 8),\n              IconButton(
                tooltip: 'Set Blueprint',
                icon: const Icon(Icons.image, color: Colors.white70),
                onPressed: _pickBlueprint,
              ),'''
)

# And add the window in the Stack at the end of the Scaffold
# Check if body is already a Stack, if not wrap it.
if 'body: LayoutBuilder(' in content:
    content = content.replace(
        'body: LayoutBuilder(',
        '''body: Stack(
          children: [
            Positioned.fill(
              child: LayoutBuilder('''
    )
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
