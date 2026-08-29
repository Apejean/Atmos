import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

target = """                                        transformationController:
                                            _transformationController,
                                        onEdit: () => _editSpeaker(node),
                                      );"""

replacement = """                                        transformationController:
                                            _transformationController,
                                        onEdit: () => _editSpeaker(node),
                                        onSpeakerSelected: (id) => setState(() => _selectedInspectorSpeakerId = id),
                                      );"""

content = content.replace(target, replacement)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)

