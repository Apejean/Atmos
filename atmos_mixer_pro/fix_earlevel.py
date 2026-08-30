import os

path = '/Users/Allweno/Projects/GitHub/atmos/atmos_mixer_pro/lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart'
with open(path, 'r') as f:
    content = f.read()

content = content.replace(
    '"earLevel": widget.activeRoom?.earLevel ?? bp.earLevelMeters,',
    '"earLevel": widget.activeRoom?.earLevel ?? 1.2,'
)

with open(path, 'w') as f:
    f.write(content)
