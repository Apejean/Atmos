import re

def main():
    path = "lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart"
    with open(path, "r") as f:
        content = f.read()

    # 1. We need engineState to get maxChannels
    old_build_start = """  @override
  Widget build(BuildContext context) {
    final layout = ref.watch(speakerLayoutProvider);
    final rooms = ref.watch(roomZoneProvider);
    SpeakerNode? speaker;"""
    
    new_build_start = """  @override
  Widget build(BuildContext context) {
    final layout = ref.watch(speakerLayoutProvider);
    final rooms = ref.watch(roomZoneProvider);
    final engineState = ref.watch(engineStateProvider);
    final maxChannels = engineState.outputChannelCount;
    final speakers = layout;
    SpeakerNode? speaker;"""
    content = content.replace(old_build_start, new_build_start)

    # 2. Fix the nullable issues in dropdown
    old_dropdown = """                              if (speaker.channel == i) const Padding(
                                padding: EdgeInsets.only(left: 8.0),
                                child: Icon(Icons.check, size: 16, color: Colors.lightBlueAccent),
                              )"""
    new_dropdown = """                              if (speaker!.channel == i) const Padding(
                                padding: EdgeInsets.only(left: 8.0),
                                child: Icon(Icons.check, size: 16, color: Colors.lightBlueAccent),
                              )"""
    content = content.replace(old_dropdown, new_dropdown)

    # 3. Add isFixed to _updateSpeaker signature and call
    old_update = """  void _updateSpeaker(SpeakerNode speaker, {double? x, double? y, double? z, double? pan, double? tilt, double? rot, double? disp, double? rev}) {
    ref.read(speakerLayoutProvider.notifier).updateSpeaker(speaker.copyWith(
      x: x ?? speaker.x,
      y: y ?? speaker.y,
      heightZ: z ?? speaker.heightZ,
      pitchTilt: tilt ?? speaker.pitchTilt,
      rotation: rot ?? speaker.rotation,
      panDeg: pan ?? speaker.panDeg,
      dispersionAngle: disp ?? speaker.dispersionAngle,
      reverbSend: rev ?? speaker.reverbSend,
    ));"""
    
    new_update = """  void _updateSpeaker(SpeakerNode speaker, {double? x, double? y, double? z, double? pan, double? tilt, double? rot, double? disp, double? rev, bool? isFixed}) {
    ref.read(speakerLayoutProvider.notifier).updateSpeaker(speaker.copyWith(
      x: x ?? speaker.x,
      y: y ?? speaker.y,
      heightZ: z ?? speaker.heightZ,
      pitchTilt: tilt ?? speaker.pitchTilt,
      rotation: rot ?? speaker.rotation,
      panDeg: pan ?? speaker.panDeg,
      dispersionAngle: disp ?? speaker.dispersionAngle,
      reverbSend: rev ?? speaker.reverbSend,
      isFixed: isFixed ?? speaker.isFixed,
    ));"""
    content = content.replace(old_update, new_update)

    with open(path, "w") as f:
        f.write(content)

main()
