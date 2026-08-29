import re

with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'r') as f:
    content = f.read()

content = content.replace("ceilingHeight = val.clamp(1.5, 20.0)", "ceilingHeight = val.clamp(1.5, 1000.0)")

with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'w') as f:
    f.write(content)
