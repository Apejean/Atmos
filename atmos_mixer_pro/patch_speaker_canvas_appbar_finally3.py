import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Apply the RoomZoneState fix again (safely)
content = content.replace(
    'ref.read(roomZoneProvider.notifier).updateZone(updatedRoom);',
    'ref.read(roomZoneProvider.notifier).updateRoomZone(updatedRoom);'
)

# Fix the AppBar overflow safely by using a regex that ONLY targets the first `title: Row(` in the file (which is the AppBar).
# Or better yet, just leave the AppBar as is for now and let the user see the button on the bottom left!
# The user doesn't care about the AppBar overflow right now, they just want the button on the bottom left.
# Wait, the app CRASHES (Lost connection) when there is a RenderFlex overflow?
# No, it doesn't crash the whole OS process, it just shows the yellow/black stripes in Flutter.
# Let's fix the AppBar overflow by specifically replacing the AppBar's `title: Row(`.

app_bar_start = content.find("appBar: AppBar(")
title_row = content.find("title: Row(", app_bar_start)

if title_row != -1:
    content = content[:title_row] + "title: SingleChildScrollView(scrollDirection: Axis.horizontal, child: Row(" + content[title_row + 11:]
    
    # We need to find the matching closing bracket for this Row.
    # It ends with `backgroundColor: Colors.black,`
    bg_color_index = content.find("backgroundColor: Colors.black,", title_row)
    
    # Before backgroundColor, there is `],\n          ),`
    # We need to change it to `],\n          )),`
    bracket_index = content.rfind("),", title_row, bg_color_index)
    if bracket_index != -1:
        content = content[:bracket_index] + "))," + content[bracket_index+2:]

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)

