import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    text = f.read()
    
# Let's count InteractiveViewer
print("InteractiveViewer count:", text.count("InteractiveViewer("))
