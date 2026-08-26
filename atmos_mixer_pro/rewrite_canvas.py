import re

path = "lib/features/exhibition/screens/speaker_canvas_screen.dart"
with open(path, "r") as f:
    content = f.read()

# 1. Remove _tabController
content = re.sub(r"\s*late TabController _tabController;\n", "\n", content)
content = re.sub(r"\s*_tabController = TabController\(length: 3, vsync: this\);\n\s*_tabController\.addListener\(\(\) \{\n\s*setState\(\(\) \{ _inspectorSpeakerId = null; \}\);\n\s*\}\);\n", "\n", content)

# 2. Rewrite build method
old_build_start = """  Widget build(BuildContext context) {
    final blueprint = ref.watch(blueprintProvider);

    return GestureDetector(
      onTap: () {
        _canvasFocusNode.requestFocus();
        if (!_isMeasuringScale) setState(() => _selectedRoomId = null);
      },
      child: Scaffold(
        backgroundColor: AppColors.background,
        appBar: AppBar(
          bottom: TabBar(
            controller: _tabController,
            indicatorColor: AppColors.primaryNeon,
            labelColor: AppColors.primaryNeon,
            unselectedLabelColor: Colors.white54,
            tabs: const [
              Tab(text: 'Room A: Main Hall'),
              Tab(text: 'Room B: Lobby'),
              Tab(text: 'Room C: Atmos Studio'),
            ],
          ),
          title: SingleChildScrollView(
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

new_build_start = """  Widget build(BuildContext context) {
    final blueprint = ref.watch(blueprintProvider);
    final config = ref.watch(configProvider);
    final uiRooms = config?.rooms ?? [];
    final tabLength = uiRooms.isEmpty ? 1 : uiRooms.length;

    return DefaultTabController(
      length: tabLength,
      child: Builder(
        builder: (context) {
          final tabController = DefaultTabController.of(context);
          tabController.addListener(() {
            if (tabController.indexIsChanging && _inspectorSpeakerId != null) {
              setState(() { _inspectorSpeakerId = null; });
            }
          });
          return GestureDetector(
            onTap: () {
              _canvasFocusNode.requestFocus();
              setState(() => _selectedRoomId = null);
            },
            child: Scaffold(
              backgroundColor: AppColors.background,
              appBar: AppBar(
                title: const Text('Exhibition Canvas'),
                bottom: PreferredSize(
                  preferredSize: const Size.fromHeight(100),
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Row(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          const Text('SPL HEATMAP', style: TextStyle(fontSize: 12, fontWeight: FontWeight.bold, color: Colors.white70)),
                          const SizedBox(width: 8),
                          ElevatedButton.icon(
                            onPressed: () {
                              setState(() => _showHeatmap = !_showHeatmap);
                            },
                            icon: Icon(_showHeatmap ? Icons.visibility : Icons.visibility_off, size: 16),
                            label: Text(_showHeatmap ? 'HIDE HEATMAP' : 'SHOW HEATMAP'),
                            style: ElevatedButton.styleFrom(
                              backgroundColor: const Color(0xFF23252A),
                              foregroundColor: _showHeatmap ? AppColors.primaryNeon : Colors.white54,
                            ),
                          ),
                          const SizedBox(width: 16),
                          ElevatedButton.icon(
                            onPressed: () {},
                            icon: const Icon(Icons.picture_as_pdf, size: 16),
                            label: const Text('EXPORT PDF REPORT'),
                            style: ElevatedButton.styleFrom(
                              backgroundColor: const Color(0xFF23252A),
                              foregroundColor: AppColors.primaryNeon,
                            ),
                          ),
                          const SizedBox(width: 24),
                          Consumer(
                            builder: (context, ref, child) {"""

content = content.replace(old_build_start, new_build_start)
with open(path, "w") as f:
    f.write(content)

