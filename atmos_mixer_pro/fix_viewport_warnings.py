import re

with open('lib/features/exhibition/widgets/viewport_3d/room_3d_viewport.dart', 'r') as f:
    content = f.read()

content = re.sub(r"import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state\.dart';\n", "", content)
content = re.sub(r"final rz = roomZones\.firstWhere\(\(r\) => r\.id == roomId\);\n", "", content)
content = re.sub(r"final rz = roomZones\.firstWhere\(\(r\) => r\.id == _selectedRoomId\);\n", "", content)

with open('lib/features/exhibition/widgets/viewport_3d/room_3d_viewport.dart', 'w') as f:
    f.write(content)

