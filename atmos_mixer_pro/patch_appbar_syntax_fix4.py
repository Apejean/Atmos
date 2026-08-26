import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# I see it!
# At line 1860, there is `],` closing `actions: [`
# BUT right after that is `),` closing `AppBar(` ! No, wait, there's `),` which probably closes a Row or something because my previous code had `],))),`.
# The original code ended AppBar like this:
#             ],
#           ),
#           backgroundColor: Colors.black,
#         ),

# Ah! The original code was:
# title: Row( children: [ ..., IconButton(Clear Canvas), ], ), backgroundColor: Colors.black, ),
# It was all inside `title: Row(children: [...])`
# So the `], ),` closes the `children: []` and the `Row()`.
# If I changed `title: Row(children: [` to `title: const Text('...'), actions: [`
# Then `], ),` closes `actions: []` and then... `)` closes what? AppBar? But wait, I have `), backgroundColor: Colors.black, ),`
# Let's fix lines 1860-1865.
# Change `],\n          ),\n          backgroundColor: Colors.black,\n        ),`
# to `],\n          backgroundColor: Colors.black,\n        ),`

content = content.replace(
    "],\n          ),\n          backgroundColor: Colors.black,\n        ),",
    "],\n          backgroundColor: Colors.black,\n        ),"
)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
