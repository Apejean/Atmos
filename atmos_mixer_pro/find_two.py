with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    lines = f.readlines()

count = 0
for i, line in enumerate(lines):
    count += line.count('{')
    count -= line.count('}')
    if i < 100:
        print(f"Line {i+1}: count={count} - {line.strip()}")
