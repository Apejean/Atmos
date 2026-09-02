import os
path = 'lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart'
with open(path, 'r') as f:
    content = f.read()

content = content.replace("void _updateSpeaker(SpeakerNode speaker,", "void _updateSpeaker(SpeakerNode speaker, {double? x, double? y, double? z, double? pan, double? tilt, double? rot, double? disp, double? rev, bool? isFixed}) {\n    print('[DEBUG] _updateSpeaker called: id=${speaker.id}');\n    ref.read(speakerLayoutProvider.notifier).updateSpeaker(speaker.copyWith(\n      x: x ?? speaker.x,\n      y: y ?? speaker.y,\n      heightZ: z ?? speaker.heightZ,\n      pitchTilt: tilt ?? speaker.pitchTilt,\n      rotation: rot ?? speaker.rotation,\n      panDeg: pan ?? speaker.panDeg,\n      dispersionAngle: disp ?? speaker.dispersionAngle,\n      reverbSend: rev ?? speaker.reverbSend,\n      isFixed: isFixed ?? speaker.isFixed,\n    ));\n}\n/*")

content = content.replace("  // Trigger real-time sync via global state or similar if needed.\n  }", "*/")

with open(path, 'w') as f:
    f.write(content)
