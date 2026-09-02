import os
path = 'lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart'
with open(path, 'r') as f:
    content = f.read()

content = content.replace("  void _updateSpeaker(SpeakerNode speaker, {", "  void _updateSpeaker(SpeakerNode speaker, {\n    print('[DEBUG] _updateSpeaker called: ${speaker.id}');")
with open(path, 'w') as f:
    f.write(content)
