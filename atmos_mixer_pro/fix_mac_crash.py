import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# Replace ModelViewer with a placeholder icon to fix macOS crash
pattern = r"ModelViewer\(.*?interactionPrompt: InteractionPrompt\.none,\n\s*\)"
replacement = """Icon(
                Icons.person,
                size: 80,
                color: Colors.white54,
              )"""

content = re.sub(pattern, replacement, content, flags=re.DOTALL)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)

