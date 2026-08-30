import re

def main():
    path = "lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart"
    with open(path, "r") as f:
        content = f.read()

    old_update = """  void _updateSpeaker(SpeakerNode speaker, {double? x, double? y, double? z, double? pan, double? tilt, double? rot, double? disp, double? rev}) {
    final updated = speaker.copyWith(
      x: x,
      y: y,
      heightZ: z,
      panDeg: pan,
      pitchTilt: tilt,
      rotation: rot,
      dispersionAngle: disp,
      reverbSend: rev,
    );"""

    new_update = """  void _updateSpeaker(SpeakerNode speaker, {double? x, double? y, double? z, double? pan, double? tilt, double? rot, double? disp, double? rev, bool? isFixed}) {
    final updated = speaker.copyWith(
      x: x,
      y: y,
      heightZ: z,
      panDeg: pan,
      pitchTilt: tilt,
      rotation: rot,
      dispersionAngle: disp,
      reverbSend: rev,
      isFixed: isFixed,
    );"""

    content = content.replace(old_update, new_update)

    with open(path, "w") as f:
        f.write(content)

main()
