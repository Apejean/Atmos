with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'r') as f:
    content = f.read()

content = content.replace('ref.read(roomZoneProvider.notifier).setRooms(newRooms);', 'ref.read(roomZoneProvider.notifier).updateRoomZone(updatedRoom, immediate: true);')

with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'w') as f:
    f.write(content)
