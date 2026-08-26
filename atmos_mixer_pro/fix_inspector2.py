with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    lines = f.readlines()

for i, line in enumerate(lines):
    if "Size _viewportSize" in line:
        if "_inspectorSpeakerId" not in "".join(lines[i:i+5]):
            lines.insert(i+1, "  late TabController _tabController;\n  String? _inspectorSpeakerId;\n")
        break

for i, line in enumerate(lines):
    if "class _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> {" in line:
        lines[i] = "class _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> with SingleTickerProviderStateMixin {\n"
        break

for i, line in enumerate(lines):
    if "void initState() {" in line:
        if "_tabController" not in "".join(lines[i:i+5]):
            lines.insert(i+1, "    _tabController = TabController(length: 3, vsync: this);\n    _tabController.addListener(() {\n      setState(() { _inspectorSpeakerId = null; });\n    });\n")
        break

for i, line in enumerate(lines):
    if "appBar: AppBar(" in line:
        if "bottom: TabBar(" not in "".join(lines[i:i+10]):
            lines.insert(i+1, """          bottom: TabBar(
            controller: _tabController,
            indicatorColor: AppColors.primaryNeon,
            labelColor: AppColors.primaryNeon,
            unselectedLabelColor: Colors.white54,
            tabs: const [
              Tab(text: 'Room A: Main Hall'),
              Tab(text: 'Room B: Lobby'),
              Tab(text: 'Room C: Atmos Studio'),
            ],
          ),\n""")
        break

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.writelines(lines)
