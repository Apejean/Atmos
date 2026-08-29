with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Fix default room bug
# The problem is that SpeakerCanvasScreen creates defaultRoom when config is missing
# and bypasses the SharedPreferences state if rooms is empty
old_default = """    if (rooms.isEmpty) {
      if (config != null && config.rooms.isNotEmpty) {"""
new_default = """    // Delay ensureDefaultRoom to let RoomZoneState load from prefs first
    if (rooms.isEmpty && ref.read(roomZoneProvider).isEmpty) {
      if (config != null && config.rooms.isNotEmpty) {"""

# But SharedPreferences is loaded asynchronously in build() of Notifier without await.
# In Riverpod Notifier, build() is sync if it doesn't return Future.
# So `_loadFromPrefs` finishes AFTER `build` returns empty list!
# We need to change RoomZoneState to AsyncNotifier or handle the sync loading better.
