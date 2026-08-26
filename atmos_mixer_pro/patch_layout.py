import re

path = "lib/features/exhibition/screens/speaker_canvas_screen.dart"
with open(path, "r") as f:
    content = f.read()

# Replace TabBar with dynamic tabs
old_tabs = """            tabs: const [
              Tab(text: 'Room A: Main Hall'),
              Tab(text: 'Room B: Lobby'),
              Tab(text: 'Room C: Atmos Studio'),
            ],"""
new_tabs = """            tabs: uiRooms.isEmpty 
              ? [const Tab(text: 'Default Room')]
              : uiRooms.map((r) => Tab(text: r.name)).toList(),"""
content = content.replace(old_tabs, new_tabs)

# Replace 'controller: _tabController,' inside TabBar
content = re.sub(r"\s*controller: _tabController,\s*\n", "\n", content)

# Remove the scale measurement feature
content = re.sub(r"\s*bool _isMeasuringScale = false;\n", "\n", content)
content = re.sub(r"\s*Offset\? _measureStart;\n", "\n", content)
content = re.sub(r"\s*Offset\? _measureEnd;\n", "\n", content)

# Remove the Add Room and Add Speaker buttons from the FloatingActionButton
# Actually, the user said "그냥 캔버스에 기존처럼 룸추가, 스피커추가 버튼 없애고". 
# The FloatingActionButton in speaker_canvas_screen currently has an 'Add Speaker' button.
# Let's replace the whole floatingActionButton with nothing.
fab_regex = r"\s*floatingActionButton: FloatingActionButton\.extended\([\s\S]*?onPressed:\s*_addSpeaker,[\s\S]*?\),\s*\n"
content = re.sub(fab_regex, "\n", content)

with open(path, "w") as f:
    f.write(content)
