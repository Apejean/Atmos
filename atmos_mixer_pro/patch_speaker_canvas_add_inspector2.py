import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Add _selectedSpeakerId to the state
if 'String? _selectedSpeakerId;' not in content:
    content = content.replace(
        'String? _selectedRoomId;',
        'String? _selectedRoomId;\n  String? _selectedSpeakerId;'
    )

# Also import SpeakerInspectorPanel
if 'SpeakerInspectorPanel' not in content:
    content = content.replace(
        "import 'package:atmos_mixer_pro/features/exhibition/widgets/hud/room_setup_window.dart';",
        "import 'package:atmos_mixer_pro/features/exhibition/widgets/hud/room_setup_window.dart';\nimport 'package:atmos_mixer_pro/features/exhibition/widgets/hud/speaker_inspector_panel.dart';"
    )

# Add the panel to the Stack
inspector_panel_code = """
            if (_selectedSpeakerId != null)
              Positioned(
                top: 0,
                bottom: 0,
                right: 0,
                child: SpeakerInspectorPanel(
                  speakerId: _selectedSpeakerId!,
                  onClose: () => setState(() => _selectedSpeakerId = null),
                ),
              ),
"""

if 'SpeakerInspectorPanel(' not in content:
    # Add it right before the RoomSetupWindow or just after it
    room_setup_index = content.find("if (_isRoomSetupOpen)")
    content = content[:room_setup_index] + inspector_panel_code + content[room_setup_index:]

# Now, we need to handle onSpeakerTap inside SpeakerCanvasWidget.
# Search for onSpeakerTap
# Wait, let's see how speaker taps are handled.
