import re

with open('lib/features/exhibition/state/room_zone_state.dart', 'r') as f:
    content = f.read()

content = content.replace("void addRoomZone(transmissionLossDb: 0.0, ", "void addRoomZone(")
content = content.replace("void updateRoomZone(transmissionLossDb: 0.0, ", "void updateRoomZone(")
content = content.replace("void removeRoomZone(transmissionLossDb: 0.0, ", "void removeRoomZone(")

with open('lib/features/exhibition/state/room_zone_state.dart', 'w') as f:
    f.write(content)
