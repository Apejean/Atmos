with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    lines = f.readlines()

count = 0
for i, line in enumerate(lines):
    count += line.count('{')
    count -= line.count('}')
    if count == 0:
        print(f"Reached 0 at line {i+1}: {line.strip()}")
