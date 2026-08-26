import re
import os

# I broke the `AlertDialog` somewhere down the line by blindly replacing `title: Row(` !
# There are multiple `title: Row(` in the file (like in AlertDialogs!).
# My global replace replaced them all.

os.system('git checkout 691c1d4 -- lib/features/exhibition/screens/speaker_canvas_screen.dart')

