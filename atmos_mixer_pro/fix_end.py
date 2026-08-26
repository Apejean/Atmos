import re
path = "lib/features/exhibition/screens/speaker_canvas_screen.dart"
with open(path, "r") as f:
    content = f.read()

bad = """          ), // Row
        ), // Container
      ),
      ), // DefaultTabController
    );
  }
}

class _DraggableSpeakerWidget"""

good = """          ), // Row
        ), // Container
      ), // Scaffold
      ), // DefaultTabController
    ); // GestureDetector
  }
}

class _DraggableSpeakerWidget"""

content = content.replace(bad, good)
with open(path, "w") as f:
    f.write(content)
