import re

path = "lib/features/exhibition/screens/speaker_canvas_screen.dart"
with open(path, "r") as f:
    content = f.read()

# Replace the closing of the scaffold
old_end = """          ), // Row
        ), // Container
      ), // Scaffold
    ); // GestureDetector
  }
}"""
new_end = """          ), // Row
        ), // Container
      ); // Scaffold
      }) // Builder
      ), // DefaultTabController
    ); // GestureDetector
  }
}"""

content = content.replace(old_end, new_end)
with open(path, "w") as f:
    f.write(content)

