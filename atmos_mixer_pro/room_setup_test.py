import re

with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'r') as f:
    content = f.read()

# Let's check what I did to `RoomSetupWindow`
print(content[:500])
