import re

with open("lib/features/exhibition/screens/speaker_canvas_screen.dart", "r") as f:
    content = f.read()

# Delete unused _addTrajectory completely
content = re.sub(r"\s*void _addTrajectory\(\) \{\n\s*//\s*Not implemented\n\s*\}", "", content)
content = re.sub(r"\s*void _addTrajectory\(\) \{[\s\S]*?\}\n", "", content)

with open("lib/features/exhibition/screens/speaker_canvas_screen.dart", "w") as f:
    f.write(content)
