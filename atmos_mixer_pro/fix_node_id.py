import re
path = "lib/features/exhibition/screens/speaker_canvas_screen.dart"
with open(path, "r") as f:
    content = f.read()

# SpeakerNode might not have 'name', it has 'id'. Let's check `speaker_node.dart`.
# For now just use `id.substring(0, 3)` or similar if it doesn't have name. Let's just use `id` for now to satisfy compiler.
content = content.replace("node.name", "node.id")

with open(path, "w") as f:
    f.write(content)
