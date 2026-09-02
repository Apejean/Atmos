import os
path = 'lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart'
with open(path, 'r') as f:
    content = f.read()

content = content.replace("  void _updateSpeaker(SpeakerNode speaker, {\n    print('[DEBUG] _updateSpeaker called: ${speaker.id}');double? x, double? y, double? z, double? pan, double? tilt, double? rot, double? disp, double? rev, bool? isFixed}) {", "  void _updateSpeaker(SpeakerNode speaker, {double? x, double? y, double? z, double? pan, double? tilt, double? rot, double? disp, double? rev, bool? isFixed}) {\n    print('[DEBUG] _updateSpeaker called: ${speaker.id}');")
with open(path, 'w') as f:
    f.write(content)
