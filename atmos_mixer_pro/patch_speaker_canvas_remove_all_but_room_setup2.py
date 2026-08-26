import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Let's replace the whole `appBar: AppBar(...)` with a minimal one.
appbar_start = content.find("appBar: AppBar(")
appbar_end = content.find("        body: Stack(", appbar_start)

minimal_appbar = """appBar: AppBar(
          title: const Text('Exhibition Canvas'),
          backgroundColor: Colors.black,
        ),
"""

content = content[:appbar_start] + minimal_appbar + content[appbar_end:]

# Also remove floatingActionButton since they said "기능 및 버튼 UI만 남기고 싹다 지워줘"
# Actually, the floatingActionButton was used for adding speakers, which might be needed?
# "Room Setup 기능 및 버튼 UI만 남기고 싹다 지워줘 새로 구현해야해"
# Ok, I will remove floatingActionButton too.
fab_start = content.find("floatingActionButton: PopupMenuButton<String>(")
if fab_start != -1:
    # find the end of the floatingActionButton
    fab_end = content.find("    ); // GestureDetector", fab_start)
    if fab_end != -1:
        # keep the `); // GestureDetector` part
        content = content[:fab_start] + content[fab_end:]

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
