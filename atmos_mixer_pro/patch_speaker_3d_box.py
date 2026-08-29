import re

with open('lib/features/exhibition/widgets/viewport_3d/speaker_3d_box.dart', 'r') as f:
    content = f.read()

content = content.replace("..setEntry(3, 2, 0.001) // perspective", "// perspective handled by parent")

with open('lib/features/exhibition/widgets/viewport_3d/speaker_3d_box.dart', 'w') as f:
    f.write(content)
