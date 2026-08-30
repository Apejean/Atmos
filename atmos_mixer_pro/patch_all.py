import os

# 1. Update dynamic_3d_room.dart
path_room = 'lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart'
with open(path_room, 'r') as f:
    content_room = f.read()

content_room = content_room.replace(
    '"height": roomHeight,',
    '"height": roomHeight,\n        "earLevel": widget.activeRoom?.earLevel ?? bp.earLevelMeters,'
)

content_room = content_room.replace(
    '"maxSPL": s.maxSPL,',
    '"maxSPL": s.maxSPL,\n        "isFixed": s.isFixed,'
)

with open(path_room, 'w') as f:
    f.write(content_room)

# 2. Update studio_engine.html
path_html = 'assets/3d_simulator/studio_engine.html'
with open(path_html, 'r') as f:
    content_html = f.read()

content_html = content_html.replace(
    '''      if (data.room) {
        currentRoom = data.room;
        buildRoom(currentRoom.width, currentRoom.depth, currentRoom.height);
      }''',
    '''      if (data.room) {
        currentRoom = data.room;
        buildRoom(currentRoom.width, currentRoom.depth, currentRoom.height);
        buildListenerMannequin(currentRoom.earLevel || 1.2);
      }'''
)

with open(path_html, 'w') as f:
    f.write(content_html)

# 3. Update speaker_inspector_panel.dart
path_inspector = 'lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart'
with open(path_inspector, 'r') as f:
    content_inspector = f.read()

content_inspector = content_inspector.replace(
    'Function(double) onChanged,',
    'Function(double)? onChanged,'
)

content_inspector = content_inspector.replace(
    '''                    onDoubleTap: () {
                      _showEditDialog(label, value, min, max, onChanged);
                    },''',
    '''                    onDoubleTap: onChanged == null ? null : () {
                      _showEditDialog(label, value, min, max, onChanged!);
                    },'''
)

content_inspector = content_inspector.replace(
    '''                onDoubleTap: () {
                  final middle = (min + max) / 2;
                  onChanged(middle);
                },''',
    '''                onDoubleTap: onChanged == null ? null : () {
                  final middle = (min + max) / 2;
                  onChanged!(middle);
                },'''
)

content_inspector = content_inspector.replace(
    "_buildControlBox('assets/3d_simulator/icons/icon_x.svg', 'X Position', speaker.x, 'm', -roomW/2 + 0.25, roomW/2 - 0.25, (v) => _updateSpeaker(speaker!, x: v)),",
    "_buildControlBox('assets/3d_simulator/icons/icon_x.svg', 'X Position', speaker.x, 'm', 0.25, roomW - 0.25, speaker.isFixed ? null : (v) => _updateSpeaker(speaker!, x: v)),"
)
content_inspector = content_inspector.replace(
    "_buildControlBox('assets/3d_simulator/icons/icon_y.svg', 'Y Position', speaker.y, 'm', -roomD/2 + 0.25, roomD/2 - 0.25, (v) => _updateSpeaker(speaker!, y: v)),",
    "_buildControlBox('assets/3d_simulator/icons/icon_y.svg', 'Y Position', speaker.y, 'm', 0.25, roomD - 0.25, speaker.isFixed ? null : (v) => _updateSpeaker(speaker!, y: v)),"
)
content_inspector = content_inspector.replace(
    "_buildControlBox('assets/3d_simulator/icons/icon_height.svg', 'Z Height', speaker.heightZ, 'm', 0.25, roomH - 0.25, (v) => _updateSpeaker(speaker!, z: v)),",
    "_buildControlBox('assets/3d_simulator/icons/icon_height.svg', 'Z Height', speaker.heightZ, 'm', 0.25, roomH - 0.25, speaker.isFixed ? null : (v) => _updateSpeaker(speaker!, z: v)),"
)
content_inspector = content_inspector.replace(
    "_buildControlBox('assets/3d_simulator/icons/icon_tilt.svg', 'Yaw (Rotation)', speaker.rotation, '°', -180.0, 180.0, (v) => _updateSpeaker(speaker!, rot: v)),",
    "_buildControlBox('assets/3d_simulator/icons/icon_tilt.svg', 'Yaw (Rotation)', speaker.rotation, '°', -180.0, 180.0, speaker.isFixed ? null : (v) => _updateSpeaker(speaker!, rot: v)),"
)
content_inspector = content_inspector.replace(
    "_buildControlBox('assets/3d_simulator/icons/icon_tilt.svg', 'Pitch (Tilt)', speaker.pitchTilt, '°', -90.0, 90.0, (v) => _updateSpeaker(speaker!, tilt: v)),",
    "_buildControlBox('assets/3d_simulator/icons/icon_tilt.svg', 'Pitch (Tilt)', speaker.pitchTilt, '°', -90.0, 90.0, speaker.isFixed ? null : (v) => _updateSpeaker(speaker!, tilt: v)),"
)
content_inspector = content_inspector.replace(
    "_buildControlBox('assets/3d_simulator/icons/icon_dispersion.svg', 'Dispersion', speaker.dispersionAngle, '°', 10.0, 180.0, (v) => _updateSpeaker(speaker!, disp: v)),",
    "_buildControlBox('assets/3d_simulator/icons/icon_dispersion.svg', 'Dispersion', speaker.dispersionAngle, '°', 10.0, 180.0, speaker.isFixed ? null : (v) => _updateSpeaker(speaker!, disp: v)),"
)

with open(path_inspector, 'w') as f:
    f.write(content_inspector)

print("Patch applied successfully.")
