import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Replace RoomSetupWindow instantiation to pass the current room and handle onApply
room_setup_call = """                child: RoomSetupWindow(
                  onClose: () => setState(() => _isRoomSetupOpen = false),
                ),"""

# We need to find the currently selected room or just the first room for now if none selected.
# Wait, SpeakerCanvasScreen has `final blueprint = ref.watch(blueprintProvider);`
# `final selectedRoom = _selectedRoomId != null ? blueprint.rooms.firstWhere((r) => r.id == _selectedRoomId, orElse: () => blueprint.rooms.first) : (blueprint.rooms.isNotEmpty ? blueprint.rooms.first : null);`
# But let's just do it directly.

room_setup_connected = """                child: Consumer(
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

content = content.replace(room_setup_call, room_setup_connected)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
