import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Add _isRoomSetupOpen to _SpeakerCanvasScreenState
content = content.replace(
    'class _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> {',
    'class _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> {\n  bool _isRoomSetupOpen = true;'
)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
