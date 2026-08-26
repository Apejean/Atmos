import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# When I did git checkout, it reverted to BEFORE I fixed `updateRoomZone`.
# So I need to apply the `updateRoomZone` fix again.

content = content.replace(
    'ref.read(roomZoneProvider.notifier).updateZone(updatedRoom);',
    'ref.read(roomZoneProvider.notifier).updateRoomZone(updatedRoom);'
)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)

