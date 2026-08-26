import re
import os

path = "lib/features/exhibition/screens/speaker_canvas_screen.dart"
with open(path, "r") as f:
    content = f.read()

# 1. Remove _tabController
content = re.sub(r"\s*late TabController _tabController;\s*\n", "\n", content)
content = re.sub(r"\s*_tabController = TabController\(length: 3, vsync: this\);\s*\n\s*_tabController\.addListener\(\(\) \{\s*\n\s*setState\(\(\) \{ _inspectorSpeakerId = null; \}\);\s*\n\s*\}\);\s*\n", "\n", content)
content = re.sub(r"\s*controller: _tabController,\s*\n", "\n", content)

# 2. Add configProvider watch and wrap Scaffold in DefaultTabController
build_start = "  Widget build(BuildContext context) {"
build_new = """  Widget build(BuildContext context) {
    final config = ref.watch(configProvider);
    final uiRooms = config?.rooms ?? [];
    final tabLength = uiRooms.isEmpty ? 1 : uiRooms.length;
"""
content = content.replace(build_start, build_new)

# 3. Wrap Scaffold and fix AppBar tabs
scaffold_start = "child: Scaffold("
scaffold_new = """child: DefaultTabController(
        length: tabLength,
        child: Builder(builder: (context) {
          final tabController = DefaultTabController.of(context);
          // Listen to tab changes to hide inspector
          tabController.addListener(() {
            if (tabController.indexIsChanging && _inspectorSpeakerId != null) {
              setState(() { _inspectorSpeakerId = null; });
            }
          });
          return Scaffold("""
content = content.replace(scaffold_start, scaffold_new)

# 4. Replace hardcoded tabs with dynamic ones based on uiRooms
tabs_old = """            tabs: const [
              Tab(text: 'Room A: Main Hall'),
              Tab(text: 'Room B: Lobby'),
              Tab(text: 'Room C: Atmos Studio'),
            ],"""
tabs_new = """            tabs: uiRooms.isEmpty 
              ? [const Tab(text: 'Default Room')]
              : uiRooms.map((r) => Tab(text: r.name)).toList(),"""
content = content.replace(tabs_old, tabs_new)

# 5. Fix AppBar layout: move buttons below "Exhibition Canvas", and tabs below buttons
# Currently: AppBar has title (Row), bottom (TabBar).
# User wants: title (Text), bottom (Column with buttons then TabBar)

appbar_old = """          title: SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(
              children: [
                const Text('Exhibition Canvas'),
                const SizedBox(width: 24),
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  const Text('SPL HEATMAP', style: TextStyle(fontSize: 12, fontWeight: FontWeight.bold, color: Colors.white70)),
                  const SizedBox(width: 8),
                  Switch(
                    value: false,
                    onChanged: (val) {},
                    activeColor: AppColors.primaryNeon,
                  ),
                  const SizedBox(width: 16),
                  ElevatedButton.icon(
                    onPressed: () {},
                    icon: const Icon(Icons.picture_as_pdf, size: 16),
                    label: const Text('EXPORT PDF REPORT'),
                    style: ElevatedButton.styleFrom(
                      backgroundColor: Color(0xFF23252A),
                      foregroundColor: AppColors.primaryNeon,
                    ),
                  ),
                  const SizedBox(width: 16),
                ],
              ),
              const SizedBox(width: 24),
              Consumer(
                builder: (context, ref, child) {"""

appbar_new = """          title: Row(
            children: [
              const Text('Exhibition Canvas'),
              const Spacer(),
              Consumer(
                builder: (context, ref, child) {"""
content = content.replace(appbar_old, appbar_new)

# Now inject the buttons into the bottom of AppBar
bottom_old = """          bottom: TabBar(
            indicatorColor: AppColors.primaryNeon,
            labelColor: AppColors.primaryNeon,
            unselectedLabelColor: Colors.white54,
            tabs: uiRooms.isEmpty 
              ? [const Tab(text: 'Default Room')]
              : uiRooms.map((r) => Tab(text: r.name)).toList(),
          ),"""

bottom_new = """          bottom: PreferredSize(
            preferredSize: const Size.fromHeight(100),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    ElevatedButton.icon(
                      onPressed: () {},
                      icon: const Icon(Icons.picture_as_pdf, size: 16),
                      label: const Text('EXPORT PDF REPORT'),
                      style: ElevatedButton.styleFrom(
                        backgroundColor: Color(0xFF23252A),
                        foregroundColor: AppColors.primaryNeon,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                TabBar(
                  indicatorColor: AppColors.primaryNeon,
                  labelColor: AppColors.primaryNeon,
                  unselectedLabelColor: Colors.white54,
                  tabs: uiRooms.isEmpty 
                    ? [const Tab(text: 'Default Room')]
                    : uiRooms.map((r) => Tab(text: r.name)).toList(),
                ),
              ],
            ),
          ),"""

content = content.replace(bottom_old, bottom_new)

# Close the builder and DefaultTabController
scaffold_end = """        floatingActionButton: FloatingActionButton.extended("""
scaffold_end_new = """        floatingActionButton: FloatingActionButton.extended("""
content = content.replace("        floatingActionButton:", "        });\n        return Scaffold(\n".replace("return Scaffold(", " // ") + "\n        floatingActionButton:")
# Wait, I need to make sure I add `});})` at the end of the `DefaultTabController` child.
# Let's fix Scaffold closing properly.
# Find the end of `Scaffold(` block.
# Actually, replacing `child: Scaffold(` with `child: DefaultTabController(... child: Builder(... return Scaffold(` means we added 2 levels of nesting.
# So `)` at the end of `Scaffold` needs to become `);}));`

# A safer way to fix the builder closing:
content = re.sub(r"(\s*)\); // end scaffold", r"\1);\1});\1})", content + "\n // end scaffold")
# But Scaffold doesn't have `// end scaffold` comment. Let's find the exact end of Scaffold.
# It's right before `  }` of build method.
