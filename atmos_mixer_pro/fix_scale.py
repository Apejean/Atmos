with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

bad_scale = "scale: '${roomWidth / 6.0} ${roomHeight / 3.0} ${roomDepth / 4.5}',"
# 4.0 width, 2.616 height, 4.0 depth are the original bounds
good_scale = "scale: '${roomWidth / 4.016} ${roomHeight / 2.616} ${roomDepth / 4.016}',"

content = content.replace(bad_scale, good_scale)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
