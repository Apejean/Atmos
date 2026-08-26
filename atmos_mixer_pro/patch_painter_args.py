import re

path = "lib/features/exhibition/screens/speaker_canvas_screen.dart"
with open(path, "r") as f:
    content = f.read()

# Fix the painter arguments for _HeatmapPainter
bad_call = """                                      painter: _HeatmapPainter(
                                        nodes: nodes,
                                        rooms: rooms,
                                        selectedOctave: _selectedOctaveFilter,
                                      ),"""

good_call = """                                      painter: _HeatmapPainter(
                                        speakers: nodes,
                                        scale: 1.0,
                                      ),"""

content = content.replace(bad_call, good_call)

# Fix positionX, positionY to x, y
content = content.replace("spk.positionX", "spk.x")
content = content.replace("spk.positionY", "spk.y")

with open(path, "w") as f:
    f.write(content)
