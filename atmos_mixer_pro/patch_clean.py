import re

path = "lib/features/exhibition/screens/speaker_canvas_screen.dart"
with open(path, "r") as f:
    content = f.read()

# 1. Remove all emojis
content = content.replace("🌡️ SPL Heatmap", "SPL HEATMAP")
content = content.replace("📄 Export PDF Report", "EXPORT PDF REPORT")
content = content.replace("📄 Export Rigging Report", "EXPORT RIGGING REPORT")
content = content.replace("🎯 Apply 3D Calibration", "APPLY 3D CALIBRATION")
content = content.replace("📏 Room Setup", "ROOM SETUP")
content = content.replace("🔊 Speaker Inspector", "SPEAKER INSPECTOR")
content = content.replace("🎧 Virtual (Binaural)", "Virtual (Binaural)")
content = content.replace("🔊 Physical (Direct Out)", "Physical (Direct Out)")
content = content.replace("🌡️", "")
content = content.replace("📄", "")
content = content.replace("🎯", "")
content = content.replace("📏", "")
content = content.replace("🔊", "")
content = content.replace("🎧", "")

# 2. Fix the Scaffold structure safely
# Find `Widget build(BuildContext context) {` of _SpeakerCanvasScreenState
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

# Clean up _tabController
content = re.sub(r"\s*late TabController _tabController;\n", "\n", content)
content = re.sub(r"\s*_tabController = TabController\(length: 3, vsync: this\);\n\s*_tabController\.addListener\(\(\) \{\n\s*setState\(\(\) \{ _inspectorSpeakerId = null; \}\);\n\s*\}\);\n", "\n", content)

# 3. Fix the scaffold end safely
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

with open(path, "w") as f:
    f.write(content)
