import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Add mixin for TabController
content = content.replace("class _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> {",
"class _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> with SingleTickerProviderStateMixin {")

content = content.replace("  Size _viewportSize = const Size(800, 600);",
"""  Size _viewportSize = const Size(800, 600);
  late TabController _tabController;
  String? _inspectorSpeakerId;""")

content = content.replace("  void initState() {",
"""  void initState() {
    _tabController = TabController(length: 3, vsync: this);
    _tabController.addListener(() {
      setState(() { _inspectorSpeakerId = null; });
    });""")

content = content.replace("    _canvasFocusNode.dispose();",
"""    _canvasFocusNode.dispose();
    _tabController.dispose();""")

# Remove _editSpeaker method entirely and replace it with simple set state
pattern_edit_speaker = r"  Future<void> _editSpeaker\(SpeakerNode node\) async \{.*?\n  \}"
content = re.sub(pattern_edit_speaker, 
"""  void _editSpeaker(SpeakerNode node) {
    setState(() {
      _inspectorSpeakerId = node.id;
    });
  }""", content, flags=re.DOTALL)

# Add TabBar to AppBar
app_bar_pattern = r"        appBar: AppBar\(\n          title: Row\("
content = re.sub(app_bar_pattern,
"""        appBar: AppBar(
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
          title: Row(""", content)

# Inject Inspector & Add Speaker button into the Stack
stack_pattern = r"(              child: InteractiveViewer\([^>]+?\),\n              \),\n            \),)"
# Wait, let's find the InteractiveViewer in the build method.
