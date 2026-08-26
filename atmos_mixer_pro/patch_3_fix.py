import re

with open('lib/features/exhibition/state/blueprint_state.dart', 'r') as f:
    content = f.read()

update_method = """  void updateDimensions({
    double? canvasWidthMeters,
    double? canvasHeightMeters,
    double? roomHeightMeters,
    double? listeningHeightMeters,
  }) {
    state = state.copyWith(
      canvasWidthMeters: canvasWidthMeters,
      canvasHeightMeters: canvasHeightMeters,
      roomHeightMeters: roomHeightMeters,
      listeningHeightMeters: listeningHeightMeters,
    );
  }
"""

if "updateDimensions" not in content:
    content = content.replace("  Future<void> setOpacity", update_method + "\n  Future<void> setOpacity")

with open('lib/features/exhibition/state/blueprint_state.dart', 'w') as f:
    f.write(content)

with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'r') as f:
    hud_content = f.read()

hud_content = hud_content.replace(".update((s) => s.copyWith(canvasWidthMeters: v))", ".updateDimensions(canvasWidthMeters: v)")
hud_content = hud_content.replace(".update((s) => s.copyWith(canvasHeightMeters: v))", ".updateDimensions(canvasHeightMeters: v)")
hud_content = hud_content.replace(".update((s) => s.copyWith(roomHeightMeters: v))", ".updateDimensions(roomHeightMeters: v)")
hud_content = hud_content.replace(".update((s) => s.copyWith(listeningHeightMeters: v))", ".updateDimensions(listeningHeightMeters: v)")

with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'w') as f:
    f.write(hud_content)
