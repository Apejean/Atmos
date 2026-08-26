import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Add `final Function(String)? onSpeakerSelected;` to _DraggableSpeakerWidget
content = content.replace(
    'final VoidCallback? onEdit;',
    'final VoidCallback? onEdit;\n  final Function(String)? onSpeakerSelected;'
)
content = content.replace(
    'this.onEdit,',
    'this.onEdit,\n    this.onSpeakerSelected,'
)

# And in _DraggableSpeakerWidgetState, call onSpeakerSelected on tap.
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

# Also pass onSpeakerSelected when creating _DraggableSpeakerWidget inside the main build method.
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

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)

