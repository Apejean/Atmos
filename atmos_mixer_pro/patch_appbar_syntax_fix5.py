import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# I see the problem. The AppBar is completely broken because of my string replacement.
# Let's check out the original file from the commit 'cbeea72' (before I messed up AppBar).
# I will do a git checkout to the previous commit `cbeea72` for this file only,
# and then just manually move the button to the bottom left without touching the AppBar.

import os
os.system('git checkout cbeea72 -- lib/features/exhibition/screens/speaker_canvas_screen.dart')

