import re

def main():
    path = "lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart"
    with open(path, "r") as f:
        content = f.read()

    # The user wants clamp for sliders to exactly match ThreeJs (0.25 margin instead of 0.2)
    # Let's check the buildControlBox calls
    old_x = """_buildControlBox('assets/3d_simulator/icons/icon_x.svg', 'X Position', speaker.x, 'm', -roomW/2 + 0.2, roomW/2 - 0.2, (v) => _updateSpeaker(speaker!, x: v)),"""
    new_x = """_buildControlBox('assets/3d_simulator/icons/icon_x.svg', 'X Position', speaker.x, 'm', -roomW/2 + 0.25, roomW/2 - 0.25, (v) => _updateSpeaker(speaker!, x: v)),"""
    content = content.replace(old_x, new_x)

    old_y = """_buildControlBox('assets/3d_simulator/icons/icon_y.svg', 'Y Position', speaker.y, 'm', -roomD/2 + 0.2, roomD/2 - 0.2, (v) => _updateSpeaker(speaker!, y: v)),"""
    new_y = """_buildControlBox('assets/3d_simulator/icons/icon_y.svg', 'Y Position', speaker.y, 'm', -roomD/2 + 0.25, roomD/2 - 0.25, (v) => _updateSpeaker(speaker!, y: v)),"""
    content = content.replace(old_y, new_y)

    old_z = """_buildControlBox('assets/3d_simulator/icons/icon_height.svg', 'Z Height', speaker.heightZ, 'm', 0.2, roomH - 0.2, (v) => _updateSpeaker(speaker!, z: v)),"""
    new_z = """_buildControlBox('assets/3d_simulator/icons/icon_height.svg', 'Z Height', speaker.heightZ, 'm', 0.25, roomH - 0.25, (v) => _updateSpeaker(speaker!, z: v)),"""
    content = content.replace(old_z, new_z)

    with open(path, "w") as f:
        f.write(content)

main()
