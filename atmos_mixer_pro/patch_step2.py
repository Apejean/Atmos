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
new_tabs = """            tabs: ref.watch(configProvider)?.rooms.isEmpty ?? true
              ? [const Tab(text: 'Default Room')]
              : ref.watch(configProvider)!.rooms.map((r) => Tab(text: r.name)).toList(),"""
content = content.replace(old_tabs, new_tabs)

# Remove the Add Room and Add Speaker buttons from the FloatingActionButton
fab_regex = r"\s*floatingActionButton: FloatingActionButton\.extended\([\s\S]*?onPressed:\s*_addSpeaker,[\s\S]*?\),\s*\n"
content = re.sub(fab_regex, "\n", content)

with open(path, "w") as f:
    f.write(content)
