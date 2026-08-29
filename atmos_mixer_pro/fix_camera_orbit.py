with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()
content = content.replace("maxCameraOrbit: 'auto auto 100m',", "maxCameraOrbit: 'auto auto 2000m',")
with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
