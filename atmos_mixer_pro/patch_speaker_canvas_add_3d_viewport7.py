import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Apply the Speaker Inspector Panel changes safely without regex replacing the whole body.
content = content.replace(
    'String? _selectedRoomId;',
    'String? _selectedRoomId;\n  String? _selectedSpeakerId;'
)

content = content.replace(
    "import 'package:atmos_mixer_pro/features/exhibition/widgets/hud/room_setup_window.dart';",
    "import 'package:atmos_mixer_pro/features/exhibition/widgets/hud/room_setup_window.dart';\nimport 'package:atmos_mixer_pro/features/exhibition/widgets/hud/speaker_inspector_panel.dart';\nimport 'package:atmos_mixer_pro/features/exhibition/widgets/viewport_3d/room_3d_viewport.dart';"
)

# _DraggableSpeakerWidget modifications
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

# Add the Inspector panel AND the 3D viewport.
# Wait, I need to add 3D Viewport safely without breaking Scaffold.
# The body is currently a `Stack`. Let's wrap the Stack with an Expanded inside a Column.
# Actually, the original body is: `body: Stack( children: [ Positioned.fill( child: LayoutBuilder( ... ) ), Positioned( RoomSetupWindow ), ... ] )`
# I can just add the 3D Viewport inside the main layout. But then it would overlap?
# The spec says:
# Column(children: [ Expanded(2D Top View), Container(Divider), Expanded(3D Room Viewport) ])
# So the body SHOULD be a Column.

# Let's replace ONLY `body: Stack(` with `body: Column( children: [ Expanded(flex: 3, child: Stack(`
# And find exactly where `floatingActionButton:` starts to insert the closing tags for Column and Expanded.
# Let's check what is right before `floatingActionButton:`.

# find `floatingActionButton:`
