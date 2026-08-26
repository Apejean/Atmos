import re
import os

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# I need to see where it broke around line 1860
os.system('sed -n "1850,1870p" lib/features/exhibition/screens/speaker_canvas_screen.dart')

