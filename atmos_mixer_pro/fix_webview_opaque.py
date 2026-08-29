import re

# Okay, `opaque is not implemented on macOS` is STILL HAPPENING.
# Where is `opaque`? Let's find out!

import subprocess
import os

out = subprocess.check_output("grep -rn 'opaque' lib/", shell=True).decode('utf-8', errors='ignore')
with open("opaque_search.txt", "w") as f:
    f.write(out)

out2 = subprocess.check_output("grep -rn 'backgroundColor' lib/", shell=True).decode('utf-8', errors='ignore')
with open("bg_search.txt", "w") as f:
    f.write(out2)

