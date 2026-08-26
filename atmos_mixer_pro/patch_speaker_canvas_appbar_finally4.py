import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Let's fix the RenderSingleChildViewport exception that happened when clicking the room setup button.
# "RenderBox was not laid out: _RenderSingleChildViewport" usually happens when a SingleChildScrollView is inside a Column/Row without an Expanded/Flexible.
# Wait, did I wrap the AppBar title in a SingleChildScrollView? Yes!
# `title: SingleChildScrollView(scrollDirection: Axis.horizontal, child: Row(...))`
# The `title` of an AppBar is already constrained, but maybe it caused the issue when clicking buttons inside it.

# Let's just remove the `SingleChildScrollView` from the AppBar title to stop these exceptions.

content = content.replace(
    'title: SingleChildScrollView(scrollDirection: Axis.horizontal, child: Row(',
    'title: Row('
)

content = content.replace(
    '],\n          )),\n          backgroundColor: Colors.black,',
    '],\n          ),\n          backgroundColor: Colors.black,'
)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)

