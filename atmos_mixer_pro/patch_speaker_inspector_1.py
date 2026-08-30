import re

def main():
    path = "lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart"
    with open(path, "r") as f:
        content = f.read()

    # Remove Pan
    content = re.sub(r"\s*_buildControlBox\('assets/3d_simulator/icons/icon_pan\.svg',\s*'Pan',\s*speaker\.panDeg.*?,\s*\(v\)\s*=>\s*_updateSpeaker\(speaker!,\s*pan:\s*v\)\),", "", content)

    with open(path, "w") as f:
        f.write(content)

main()
