with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'r') as f:
    content = f.read()

content = content.replace("width = val.clamp(1.0, 50.0)", "width = val.clamp(1.0, 1000.0)")
content = content.replace("depth = val.clamp(1.0, 50.0)", "depth = val.clamp(1.0, 1000.0)")
content = content.replace("ceilingHeight = val.clamp(2.0, 15.0)", "ceilingHeight = val.clamp(2.0, 1000.0)")

with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'w') as f:
    f.write(content)
