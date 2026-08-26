import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Fix compilation errors: BlueprintData doesn't have `rooms`, it's managed by `roomZoneProvider`
# Let's fix the Consumer in speaker_canvas_screen.dart to use `roomZoneProvider`

room_setup_broken = """                child: Consumer(
                  builder: (context, ref, child) {
                    final blueprint = ref.watch(blueprintProvider);
                    if (blueprint.rooms.isEmpty) return const SizedBox.shrink();
                    final room = _selectedRoomId != null 
                        ? blueprint.rooms.firstWhere((r) => r.id == _selectedRoomId, orElse: () => blueprint.rooms.first)
                        : blueprint.rooms.first;
                    return RoomSetupWindow(
                      room: room,
                      onApply: (updatedRoom) {
                        ref.read(blueprintProvider.notifier).updateRoom(updatedRoom);
                      },
                      onClose: () => setState(() => _isRoomSetupOpen = false),
                    );
                  }
                ),"""

room_setup_fixed = """                child: Consumer(
                  builder: (context, ref, child) {
                    final rooms = ref.watch(roomZoneProvider);
                    if (rooms.isEmpty) return const SizedBox.shrink();
                    final room = _selectedRoomId != null 
                        ? rooms.firstWhere((r) => r.id == _selectedRoomId, orElse: () => rooms.first)
                        : rooms.first;
                    return RoomSetupWindow(
                      room: room,
                      onApply: (updatedRoom) {
                        ref.read(roomZoneProvider.notifier).updateRoom(updatedRoom);
                      },
                      onClose: () => setState(() => _isRoomSetupOpen = false),
                    );
                  }
                ),"""

content = content.replace(room_setup_broken, room_setup_fixed)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
