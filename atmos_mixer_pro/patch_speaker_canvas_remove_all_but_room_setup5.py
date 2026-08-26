import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# I messed up the `GestureDetector` closing tag again when replacing things.
# The error says: "Error: Can't find ')' to match '('." at line 1542, which is:
# return GestureDetector(

# Let's verify what the end of the file looks like.
import os
os.system('tail -n 20 lib/features/exhibition/screens/speaker_canvas_screen.dart')

