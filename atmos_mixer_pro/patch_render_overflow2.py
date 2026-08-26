import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Fix RenderFlex overflow in AppBar title row
# The overflow in title Row is because I added the button inside the title Row which is running out of space,
# but also the Spacer() and multiple buttons take too much width.
# In Flutter, if a Row overflows, we should probably wrap the right side in a SingleChildScrollView or just remove the Spacer.
# Or better, move the buttons to AppBar's `actions` list.
# The original code had: title: Row(children: [...]), and NO actions in AppBar.
# Let's wrap the right-side widgets in a SingleChildScrollView(scrollDirection: Axis.horizontal, child: Row(...))

# Look for `const Spacer(),` and wrap everything after it.
# Actually, it's easier to find `const Spacer(),` and replace it with `const Expanded(child: SizedBox()),`
# No, `Spacer` expands, but if children overflow it still overflows.
# Let's wrap the trailing buttons in a scrollable or just make the title row itself scrollable.

content = content.replace(
    "const Text('Exhibition Canvas'),\n              const Spacer(),",
    "const Text('Exhibition Canvas'),\n              const Spacer(),\n              // Wrapped in SingleChildScrollView to prevent overflow\n              Expanded(flex: 3, child: SingleChildScrollView(scrollDirection: Axis.horizontal, child: Row(mainAxisSize: MainAxisSize.min, children: ["
)

# And then close the Row and Expanded at the end of the title Row
content = content.replace(
    '''              IconButton(
                tooltip: 'Set Blueprint',
                icon: const Icon(Icons.image, color: Colors.white70),
                onPressed: _pickBlueprint,
              ),
              const SizedBox(width: 8),''',
    '''              IconButton(
                tooltip: 'Set Blueprint',
                icon: const Icon(Icons.image, color: Colors.white70),
                onPressed: _pickBlueprint,
              ),
              const SizedBox(width: 8),
            ],))),'''
)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
