import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Apply the Speaker Inspector Panel changes safely.
content = content.replace(
    'String? _selectedRoomId;',
    'String? _selectedRoomId;\n  String? _selectedSpeakerId;'
)

content = content.replace(
    "import 'package:atmos_mixer_pro/features/exhibition/widgets/hud/room_setup_window.dart';",
    "import 'package:atmos_mixer_pro/features/exhibition/widgets/hud/room_setup_window.dart';\nimport 'package:atmos_mixer_pro/features/exhibition/widgets/hud/speaker_inspector_panel.dart';\nimport 'package:atmos_mixer_pro/features/exhibition/widgets/viewport_3d/room_3d_viewport.dart';"
)

content = content.replace(
    'final VoidCallback? onEdit;',
    'final VoidCallback? onEdit;\n  final Function(String)? onSpeakerSelected;'
)
content = content.replace(
    'this.onEdit,',
    'this.onEdit,\n    this.onSpeakerSelected,'
)
content = content.replace(
    """                child: GestureDetector(
                  onPanStart: (details) {""",
    """                child: GestureDetector(
                  onTap: () {
                    if (widget.onSpeakerSelected != null) {
                      widget.onSpeakerSelected!(widget.node.id);
                    }
                  },
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
                                  );""",
    """                                  return _DraggableSpeakerWidget(
                                    key: ValueKey(node.id),
                                    node: node,
                                    roomColor: roomColor,
                                    isDuplicate: _isSpeakerInMultipleRooms(node),
                                    transformationController:
                                        _transformationController,
                                    onSpeakerSelected: (id) => setState(() => _selectedSpeakerId = id),
                                  );"""
)

# Now safely add the 3D Viewport by wrapping the `Stack` in an `Expanded` and `Column`.
content = content.replace(
    '''        body: Stack(
          children: [''',
    '''        body: Column(
          children: [
            Expanded(
              flex: 3,
              child: Stack(
                children: ['''
)

# Insert the end of Expanded, Divider, and 3D Viewport right before floatingActionButton:
target_end = """          ],
        ),
        floatingActionButton: PopupMenuButton<String>("""

replacement_end = """
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

content = content.replace(target_end, replacement_end)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
