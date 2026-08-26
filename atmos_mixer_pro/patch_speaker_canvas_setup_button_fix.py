import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

content = content.replace('bool _isRoomSetupOpen = false;\n  bool _isRoomSetupOpen = false;', 'bool _isRoomSetupOpen = false;')
content = content.replace('bool _isRoomSetupOpen = true;\n\n\n  bool _isRoomSetupOpen = false;', 'bool _isRoomSetupOpen = false;')
content = content.replace('bool _isRoomSetupOpen = true;', '')

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
