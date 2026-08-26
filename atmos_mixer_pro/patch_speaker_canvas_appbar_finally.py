import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Fix the AppBar RenderFlex once and for all.
# At line 1549, it has `title: Row( children: [ ... ] )`
# Let's wrap the ENTIRE row inside a SingleChildScrollView(scrollDirection: Axis.horizontal)

content = content.replace(
    'title: Row(',
    'title: SingleChildScrollView(scrollDirection: Axis.horizontal, child: Row('
)
content = content.replace(
    '],\n          ),\n          backgroundColor: Colors.black,',
    '],\n          )),\n          backgroundColor: Colors.black,'
)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)

