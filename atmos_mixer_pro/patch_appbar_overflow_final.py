import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# The user is getting RenderFlex overflowed by 217 pixels in the AppBar title Row.
# Let's completely remove the right-side widgets from `title: Row(...)` and move them to `actions: [...]`.
# That is the proper Flutter way to handle AppBar.

# Find the start of the AppBar
appbar_match = re.search(r'appBar: AppBar\(\n\s*title: Row\(\n\s*children: \[\n\s*const Text\(\'Exhibition Canvas\'\),\n\s*const SizedBox\(width: 8\),\n\s*Expanded\(child: SingleChildScrollView\(scrollDirection: Axis.horizontal, child: Row\(mainAxisSize: MainAxisSize.max, mainAxisAlignment: MainAxisAlignment.end, children: \[', content)

if appbar_match:
    # Replace the start
    content = content.replace(
        "title: Row(\n            children: [\n              const Text('Exhibition Canvas'),\n              const SizedBox(width: 8),\n              Expanded(child: SingleChildScrollView(scrollDirection: Axis.horizontal, child: Row(mainAxisSize: MainAxisSize.max, mainAxisAlignment: MainAxisAlignment.end, children: [",
        "title: const Text('Exhibition Canvas'),\n          actions: ["
    )
    # Replace the end
    content = content.replace(
        "const SizedBox(width: 8),\n            ],))),",
        "const SizedBox(width: 16),\n          ],"
    )

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
