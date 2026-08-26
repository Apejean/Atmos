import re

path = "lib/features/exhibition/screens/speaker_canvas_screen.dart"
with open(path, "r") as f:
    content = f.read()

# Make the TabController re-initialize safely if room count changes by making length dynamic.
# Actually, since it's an explicit TabController `late TabController _tabController;`, 
# changing the number of tabs in `TabBar` will throw an error if it doesn't match `_tabController.length`.

# Let's fix this cleanly by overriding the `build` method to re-instantiate or just using DefaultTabController.
# I'll just change the length in initState to a very large number? No.
# I will use a simple Builder wrapped in DefaultTabController and remove `_tabController` usages.

content = content.replace("late TabController _tabController;", "")
content = re.sub(r"\s*_tabController = TabController\(length: 3, vsync: this\);\s*\n", "\n", content)
content = re.sub(r"\s*_tabController\.addListener\(\(\) \{\s*\n\s*setState\(\(\) \{ _inspectorSpeakerId = null; \}\);\s*\n\s*\}\);\s*\n", "\n", content)
content = content.replace("controller: _tabController,", "")

build_start = """  Widget build(BuildContext context) {
    final blueprint = ref.watch(blueprintProvider);

    return GestureDetector("""

build_new = """  Widget build(BuildContext context) {
    final blueprint = ref.watch(blueprintProvider);
    final roomCount = ref.watch(configProvider)?.rooms.length ?? 1;

    return DefaultTabController(
      length: roomCount == 0 ? 1 : roomCount,
      child: GestureDetector("""

content = content.replace(build_start, build_new)

# Find the end of `GestureDetector`
end_old = """          ), // Row
        ), // Container
      ),
    );
  }
}

class _DraggableSpeakerWidget"""

end_new = """          ), // Row
        ), // Container
      ),
      ), // DefaultTabController
    );
  }
}

class _DraggableSpeakerWidget"""

content = content.replace(end_old, end_new)

with open(path, "w") as f:
    f.write(content)
