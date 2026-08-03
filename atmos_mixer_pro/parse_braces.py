import re

with open("lib/features/exhibition/screens/speaker_canvas_screen.dart", "r") as f:
    lines = f.readlines()

depth = 0
for i, line in enumerate(lines):
    if i < 900: continue
    if i > 1400: break
    for char in line:
        if char == '{':
            depth += 1
        elif char == '}':
            depth -= 1
        if depth == 0 and i > 960:
            print(f"Brace Depth hit 0 at line {i+1}: {line.strip()}")
            depth = -999999  # to avoid multiple prints
