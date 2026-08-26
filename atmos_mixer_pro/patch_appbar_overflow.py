import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Fix overflow: still overflowing by 209 pixels.
# Let's completely remove the Spacer() and use an Expanded wrapping the SingleChildScrollView to let it shrink.
# Actually, the entire right side is inside an Expanded now, but maybe flex is wrong.
# Let's change the title to a Row where only the title text takes minimal space, and the rest is wrapped in Expanded(child: SingleChildScrollView(scrollDirection: Axis.horizontal))

content = content.replace(
    "const Text('Exhibition Canvas'),\n              const Spacer(),\n              // Wrapped in SingleChildScrollView to prevent overflow\n              Expanded(flex: 3, child: SingleChildScrollView(scrollDirection: Axis.horizontal, child: Row(mainAxisSize: MainAxisSize.min, children: [",
    "const Text('Exhibition Canvas'),\n              const SizedBox(width: 8),\n              Expanded(child: SingleChildScrollView(scrollDirection: Axis.horizontal, child: Row(mainAxisSize: MainAxisSize.max, mainAxisAlignment: MainAxisAlignment.end, children: ["
)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
