with open('lib/features/exhibition/state/room_zone_state.dart', 'r') as f:
    content = f.read()

# Add a boolean to check if loading is complete
content = content.replace("class RoomZoneState extends Notifier<List<RoomZone>> {", "class RoomZoneState extends Notifier<List<RoomZone>> {\n  bool _isLoaded = false;\n  bool get isLoaded => _isLoaded;")

old_load = """      } else {
        state = [];
      }
    }"""
new_load = """      } else {
        state = [];
      }
    }
    _isLoaded = true;"""
content = content.replace(old_load, new_load)

with open('lib/features/exhibition/state/room_zone_state.dart', 'w') as f:
    f.write(content)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    screen = f.read()

old_ensure = """  void _ensureDefaultRoom() {
    final rooms = ref.read(roomZoneProvider);
    final config = ref.read(configProvider);

    if (rooms.isEmpty) {"""

new_ensure = """  void _ensureDefaultRoom() {
    final roomNotifier = ref.read(roomZoneProvider.notifier);
    if (!roomNotifier.isLoaded) {
      // Retry in 100ms
      Future.delayed(const Duration(milliseconds: 100), _ensureDefaultRoom);
      return;
    }
    
    final rooms = ref.read(roomZoneProvider);
    final config = ref.read(configProvider);

    if (rooms.isEmpty) {"""
screen = screen.replace(old_ensure, new_ensure)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(screen)
