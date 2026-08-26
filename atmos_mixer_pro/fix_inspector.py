with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

content = content.replace("  Size _viewportSize = const Size(800, 600);",
"""  Size _viewportSize = const Size(800, 600);
  late TabController _tabController;
  String? _inspectorSpeakerId;""")

# Also check for override_on_non_overriding_member on `_showRoomSetupDialog` if it has @override.
# Ah, _showRoomSetupDialog shouldn't have @override.
content = content.replace("  @override\n  Future<void> _showRoomSetupDialog() async {", "  Future<void> _showRoomSetupDialog() async {")

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
