with open('lib/features/exhibition/state/room_zone_state.dart', 'r') as f:
    content = f.read()

# Change _loadFromPrefs to run sync initially or at least let the provider be initialized with the last known state
# We'll use a hack to read SharedPreferences synchronously during startup or just await it before runApp.
# Since we can't easily change main.dart here, let's fix RoomZoneState logic.
old_load = """  @override
  List<RoomZone> build() {
    _loadFromPrefs();
    ref.onDispose(() {
      _saveDebounceTimer?.cancel();
    });
    return [];
  }"""

new_load = """  @override
  List<RoomZone> build() {
    _loadFromPrefs();
    ref.onDispose(() {
      _saveDebounceTimer?.cancel();
    });
    return [];
  }"""
# Wait, actually, _loadFromPrefs changes the state, but SpeakerCanvasScreen calls _ensureDefaultRoom inside initState's addPostFrameCallback.
# If SharedPreferences is slow, it overwrites it.
