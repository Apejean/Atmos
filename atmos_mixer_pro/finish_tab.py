import re

path = "lib/features/exhibition/screens/speaker_canvas_screen.dart"
with open(path, "r") as f:
    content = f.read()

# 1. Provide `uiRooms` in the build method.
build_start = """  Widget build(BuildContext context) {
    final blueprint = ref.watch(blueprintProvider);

    return GestureDetector("""

build_new = """  Widget build(BuildContext context) {
    final blueprint = ref.watch(blueprintProvider);
    final config = ref.watch(configProvider);
    final uiRooms = config?.rooms ?? [];
    final tabLength = uiRooms.isEmpty ? 1 : uiRooms.length;

    return DefaultTabController(
      length: tabLength,
      child: Builder(builder: (context) {
        final tabController = DefaultTabController.of(context);
        tabController.addListener(() {
          if (tabController.indexIsChanging && _inspectorSpeakerId != null) {
            setState(() { _inspectorSpeakerId = null; });
          }
        });
        return GestureDetector("""

content = content.replace(build_start, build_new)

# 2. Fix the Scaffold end
scaffold_end_old = """          ), // Row
        ), // Container
      ),
    );
  }
}

class _DraggableSpeakerWidget"""

scaffold_end_new = """          ), // Row
        ), // Container
      ),
      ); // Builder
      ), // DefaultTabController
    );
  }
}

class _DraggableSpeakerWidget"""

content = content.replace(scaffold_end_old, scaffold_end_new)

# 3. Clean up the manual _tabController from initState/dispose
content = re.sub(r"\s*late TabController _tabController;\n", "\n", content)
content = re.sub(r"\s*_tabController = TabController\(length: 3, vsync: this\);\n\s*_tabController\.addListener\(\(\) \{\n\s*setState\(\(\) \{ _inspectorSpeakerId = null; \}\);\n\s*\}\);\n", "\n", content)

with open(path, "w") as f:
    f.write(content)
