import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Replace the first `Positioned.fill(` which wraps `child: LayoutBuilder(`
# Wait, line 1870 is `Positioned.fill(`.

target = """            Positioned.fill(
              child: LayoutBuilder(
          builder: (context, constraints) {"""

replacement = """            Positioned.fill(
              child: Column(
                children: [
                  Expanded(
                    flex: 3,
                    child: LayoutBuilder(
                      builder: (context, constraints) {"""

# We also need to close the `Expanded` and `Column` at the end of the `LayoutBuilder`.
# The `LayoutBuilder` ends somewhere. Let's find where.
