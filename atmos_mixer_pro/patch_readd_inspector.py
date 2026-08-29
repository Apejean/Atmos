import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Add the inspector panel to the main Stack.
# We will place it at the very top of the Stack (end of the list of children), so it draws over everything.
target = """            if (!_isRoomSetupOpen)
              Positioned(
                left: 16,
                bottom: 16,
                child: OutlinedButton.icon(
                  icon: const Icon(Icons.picture_in_picture_alt_outlined, size: 16),
                  label: const Text('ROOM SETUP'),
                  style: OutlinedButton.styleFrom(
                    foregroundColor: Colors.lightBlueAccent,
                    backgroundColor: const Color(0xFF2C394B).withValues(alpha: 0.8),
                    side: const BorderSide(color: Colors.lightBlueAccent),
                    padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                  ),
                  onPressed: () => setState(() => _isRoomSetupOpen = true),
                ),
              ),

          ],
        ),
      ),
    );"""

replacement = """            if (!_isRoomSetupOpen)
              Positioned(
                left: 16,
                bottom: 16,
                child: OutlinedButton.icon(
                  icon: const Icon(Icons.picture_in_picture_alt_outlined, size: 16),
                  label: const Text('ROOM SETUP'),
                  style: OutlinedButton.styleFrom(
                    foregroundColor: Colors.lightBlueAccent,
                    backgroundColor: const Color(0xFF2C394B).withValues(alpha: 0.8),
                    side: const BorderSide(color: Colors.lightBlueAccent),
                    padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                  ),
                  onPressed: () => setState(() => _isRoomSetupOpen = true),
                ),
              ),
            if (_selectedInspectorSpeakerId != null)
              Positioned(
                top: 0,
                bottom: 0,
                right: 0,
                child: SpeakerInspectorPanel(
                  speakerId: _selectedInspectorSpeakerId!,
                  onClose: () => setState(() => _selectedInspectorSpeakerId = null),
                ),
              ),
          ],
        ),
      ),
    );"""

content = content.replace(target, replacement)

# Now make sure `_DraggableSpeakerWidget` triggers the selection!
# The `onTap` for `_DraggableSpeakerWidget` was removed earlier. Let's add it back.
target2 = """              Positioned.fill(
                child: GestureDetector(
                  onPanStart: (_) => setState(() => _isDragging = true),"""
replacement2 = """              Positioned.fill(
                child: GestureDetector(
                  onTap: () {
                    if (widget.onSpeakerSelected != null) {
                      widget.onSpeakerSelected!(widget.node.id);
                    }
                  },
                  onPanStart: (_) => setState(() => _isDragging = true),"""

content = content.replace(target2, replacement2)

# Also need to add `onSpeakerSelected` to `_DraggableSpeakerWidget` class definition.
target3 = """class _DraggableSpeakerWidget extends ConsumerStatefulWidget {
  final SpeakerNode node;
  final Color? roomColor;
  final bool isDuplicate;
  final TransformationController transformationController;
  final VoidCallback? onEdit;"""
replacement3 = """class _DraggableSpeakerWidget extends ConsumerStatefulWidget {
  final SpeakerNode node;
  final Color? roomColor;
  final bool isDuplicate;
  final TransformationController transformationController;
  final VoidCallback? onEdit;
  final Function(String)? onSpeakerSelected;"""
content = content.replace(target3, replacement3)

target4 = """    required this.transformationController,
    this.onEdit,
  });"""
replacement4 = """    required this.transformationController,
    this.onEdit,
    this.onSpeakerSelected,
  });"""
content = content.replace(target4, replacement4)

# And finally pass it when creating `_DraggableSpeakerWidget`
target5 = """                                      return _DraggableSpeakerWidget(
                                        key: ValueKey(node.id),
                                        node: node,
                                        roomColor:
                                            roomColor ?? AppColors.primaryNeon,
                                        isDuplicate: isDuplicate,
                                        transformationController:
                                            _transformationController,
                                      );"""
replacement5 = """                                      return _DraggableSpeakerWidget(
                                        key: ValueKey(node.id),
                                        node: node,
                                        roomColor:
                                            roomColor ?? AppColors.primaryNeon,
                                        isDuplicate: isDuplicate,
                                        transformationController:
                                            _transformationController,
                                        onSpeakerSelected: (id) => setState(() => _selectedInspectorSpeakerId = id),
                                      );"""
content = content.replace(target5, replacement5)

# Oh, also need to uncomment the import of SpeakerInspectorPanel.
# But I already did in the previous revert patch script. Let's make sure it's there.
# It is!

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)

