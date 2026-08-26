import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Add Inspector UI + Add Speaker FAB to the main Scaffold Stack
# Let's find: `              ), // Stack` right before `        floatingActionButton: Column(`

stack_end_pattern = r"(              \),\n            \),\n          \],\.?\n        \),\n        floatingActionButton: Column\()"
# Actually, the structure is:
# Scaffold(
#   body: Stack(
#     children: [
#       InteractiveViewer(...),
#       // Toolbar, sidebars etc.
#     ],
#   ),
#   floatingActionButton: ...

