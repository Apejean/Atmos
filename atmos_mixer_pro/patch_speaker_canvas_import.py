import re

def main():
    path = "lib/features/exhibition/screens/speaker_canvas_screen.dart"
    with open(path, "r") as f:
        content = f.read()

    old_import = "import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';"
    new_import = "import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';\nimport 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';"

    content = content.replace(old_import, new_import)

    # also remove the unnecessary null check activeRoom != null since we are already inside a condition or activeRoom is non-nullable.
    # The warning was: The operand can't be 'null', so the condition is always 'true'. Remove the condition • lib/features/exhibition/screens/speaker_canvas_screen.dart:176:34
    content = content.replace("if (activeRoom != null && activeRoom.physicalWidth > 0 && activeRoom.physicalHeight > 0)", "if (activeRoom.physicalWidth > 0 && activeRoom.physicalHeight > 0)")
    
    with open(path, "w") as f:
        f.write(content)

main()
