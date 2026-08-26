import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# The method name is `updateRoomZone` ! Not updateZone or updateRoom.

content = content.replace(
    'ref.read(roomZoneProvider.notifier).updateZone(updatedRoom);',
    'ref.read(roomZoneProvider.notifier).updateRoomZone(updatedRoom);'
)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
