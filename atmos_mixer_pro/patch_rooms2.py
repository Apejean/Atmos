import re

# 1. Fix RoomSetupWindow
with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'r') as f:
    content = f.read()

content = content.replace('blueprint.rooms', 'ref.read(roomZoneProvider)')
content = content.replace('blueprintProvider.notifier', 'roomZoneProvider.notifier')
content = content.replace('updateRooms', 'setRooms') # Check if it's setRooms or updateState

with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'w') as f:
    f.write(content)

# 2. Fix Room3DViewport
with open('lib/features/exhibition/widgets/viewport_3d/room_3d_viewport.dart', 'r') as f:
    content = f.read()

content = content.replace('BlueprintState blueprint', 'BlueprintData blueprint')
content = content.replace('final blueprint = ref.watch(blueprintProvider);', 'final blueprint = ref.watch(blueprintProvider);\n    final rooms = ref.watch(roomZoneProvider);')

content = re.sub(
    r'_IsoRoomPainter\(\s*neonCyan: neonCyan,\s*blueprint: blueprint,\s*orbitAngleX: _orbitAngleX,\s*orbitAngleY: _orbitAngleY,\s*\)',
    r'_IsoRoomPainter(neonCyan: neonCyan, blueprint: blueprint, rooms: rooms, orbitAngleX: _orbitAngleX, orbitAngleY: _orbitAngleY)',
    content
)
content = re.sub(
    r'_ThermalHeatmapPainter\(\s*speakers: speakers,\s*blueprint: blueprint,\s*orbitAngleX: _orbitAngleX,\s*orbitAngleY: _orbitAngleY,\s*targetSPL: widget.targetSPL,\s*\)',
    r'_ThermalHeatmapPainter(speakers: speakers, blueprint: blueprint, rooms: rooms, orbitAngleX: _orbitAngleX, orbitAngleY: _orbitAngleY, targetSPL: widget.targetSPL)',
    content
)

content = content.replace('class _IsoRoomPainter extends CustomPainter {', 'class _IsoRoomPainter extends CustomPainter {\n  final List<RoomZone> rooms;')
content = content.replace('_IsoRoomPainter({', '_IsoRoomPainter({required this.rooms,')
content = content.replace('blueprint.rooms', 'rooms')

content = content.replace('class _ThermalHeatmapPainter extends CustomPainter {', 'class _ThermalHeatmapPainter extends CustomPainter {\n  final List<RoomZone> rooms;')
content = content.replace('_ThermalHeatmapPainter({', '_ThermalHeatmapPainter({required this.rooms,')

# Fix cy vs dy issue in project
content = content.replace('floorCenter.cy', 'floorCenter.dy')

with open('lib/features/exhibition/widgets/viewport_3d/room_3d_viewport.dart', 'w') as f:
    f.write(content)

