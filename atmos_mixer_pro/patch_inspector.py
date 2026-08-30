import re

with open('lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart', 'r') as f:
    content = f.read()

old_logic = """    if (rooms.isNotEmpty) {
      roomW = rooms.first.physicalWidth;
      roomD = rooms.first.physicalHeight;
      roomH = rooms.first.ceilingHeight;
    }"""

new_logic = """    if (rooms.isNotEmpty) {
      final activeRoom = rooms.firstWhere((r) => r.id == speaker?.roomId, orElse: () => rooms.first);
      roomW = activeRoom.physicalWidth;
      roomD = activeRoom.physicalHeight;
      roomH = activeRoom.ceilingHeight;
    }"""

if old_logic in content:
    content = content.replace(old_logic, new_logic)
    with open('lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart', 'w') as f:
        f.write(content)
    print("Patched inspector room bounds successfully")
else:
    print("Old logic not found")
