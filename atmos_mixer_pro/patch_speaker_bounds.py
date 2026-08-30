import re

def main():
    path = "lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart"
    with open(path, "r") as f:
        content = f.read()

    # Z height max bound in inspector was roomH.
    # We should subtract a small value or base it on a fixed speaker size (e.g. 0.4m)
    # Let's say speaker physical height is ~0.4m. So half height is 0.2m. 
    # Max Z = roomH - 0.2
    
    # We also need to fix X, Y bounds 
    # X Position: min = -roomW/2 + 0.2, max = roomW/2 - 0.2
    # Y Position: min = -roomD/2 + 0.2, max = roomD/2 - 0.2
    
    content = content.replace(
        "'-roomW/2, roomW/2'",
        "'-roomW/2 + 0.2, roomW/2 - 0.2'"
    )
    content = content.replace(
        "-roomW/2, roomW/2,",
        "-roomW/2 + 0.2, roomW/2 - 0.2,"
    )
    content = content.replace(
        "-roomD/2, roomD/2,",
        "-roomD/2 + 0.2, roomD/2 - 0.2,"
    )
    content = content.replace(
        "0.0, roomH,",
        "0.2, roomH - 0.2,"
    )

    with open(path, "w") as f:
        f.write(content)

main()
