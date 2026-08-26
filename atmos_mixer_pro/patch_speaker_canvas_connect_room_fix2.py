import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Fix compilation errors: RoomZoneState has `updateZone(RoomZone)` not `updateRoom(RoomZone)`

content = content.replace(
    'ref.read(roomZoneProvider.notifier).updateRoom(updatedRoom);',
    'ref.read(roomZoneProvider.notifier).updateZone(updatedRoom);'
)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
