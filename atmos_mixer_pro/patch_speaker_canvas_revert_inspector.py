import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# The user requested to remove everything EXCEPT the Room Setup UI and Button.
# This means removing the Speaker Inspector Panel, the 3D Viewport, and restoring the body to its previous state (just the LayoutBuilder with the Stack).

# 1. Remove Imports
content = content.replace(
    "import 'package:atmos_mixer_pro/features/exhibition/widgets/hud/speaker_inspector_panel.dart';\nimport 'package:atmos_mixer_pro/features/exhibition/widgets/viewport_3d/room_3d_viewport.dart';",
    ""
)

# 2. Remove Inspector properties
content = content.replace(
    'String? _selectedRoomId;\n  String? _selectedSpeakerId;',
    'String? _selectedRoomId;'
)

# 3. Restore _DraggableSpeakerWidget
content = content.replace(
    'final VoidCallback? onEdit;\n  final Function(String)? onSpeakerSelected;',
    'final VoidCallback? onEdit;'
)
content = content.replace(
    'this.onEdit,\n    this.onSpeakerSelected,',
    'this.onEdit,'
)
content = content.replace(
    """                child: GestureDetector(
                  onTap: () {
                    if (widget.onSpeakerSelected != null) {
                      widget.onSpeakerSelected!(widget.node.id);
                    }
                  },
                  onPanStart: (details) {""",
    """                child: GestureDetector(
                  onPanStart: (details) {"""
)
content = content.replace(
    """                                  return _DraggableSpeakerWidget(
                                    key: ValueKey(node.id),
                                    node: node,
                                    roomColor: roomColor,
                                    isDuplicate: _isSpeakerInMultipleRooms(node),
                                    transformationController:
                                        _transformationController,
                                    onSpeakerSelected: (id) => setState(() => _selectedSpeakerId = id),
                                  );""",
    """                                  return _DraggableSpeakerWidget(
                                    key: ValueKey(node.id),
                                    node: node,
                                    roomColor: roomColor,
                                    isDuplicate: _isSpeakerInMultipleRooms(node),
                                    transformationController:
                                        _transformationController,
                                  );"""
)

# 4. Restore Scaffold body
target_body = '''        body: Column(
          children: [
            Expanded(
              flex: 3,
              child: Stack(
                children: ['''
replacement_body = '''        body: Stack(
          children: ['''
content = content.replace(target_body, replacement_body)

# 5. Remove Inspector and 3D Viewport from body
target_end = """
            if (_selectedSpeakerId != null)
              Positioned(
                top: 0,
                bottom: 0,
                right: 0,
                child: SpeakerInspectorPanel(
                  speakerId: _selectedSpeakerId!,
                  onClose: () => setState(() => _selectedSpeakerId = null),
                ),
              ),
          ],
        ),
        ),
        Container(height: 2, color: const Color(0xFF3F556D)),
        const Expanded(
          flex: 2,
          child: Room3DViewport(),
        ),
        ],
        ),
        floatingActionButton: PopupMenuButton<String>("""

replacement_end = """          ],
        ),
        floatingActionButton: PopupMenuButton<String>("""

content = content.replace(target_end, replacement_end)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
