import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Let's inspect the `SpeakerCanvasWidget` or where the speakers are rendered.
import os
os.system('cat lib/features/exhibition/screens/speaker_canvas_screen.dart | grep -n "class SpeakerCanvasWidget"')

