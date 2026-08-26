import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Let's see how `build` method ends.
# I will use grep to find the line `  @override\n  Widget build(BuildContext context) {`
import os
os.system("cat lib/features/exhibition/screens/speaker_canvas_screen.dart | grep -n -A 30 'Widget build(BuildContext context)'")

