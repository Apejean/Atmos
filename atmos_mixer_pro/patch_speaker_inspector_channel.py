import re

def main():
    path = "lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart"
    with open(path, "r") as f:
        content = f.read()

    # Add missing import for engine state if not exists
    if "import 'package:atmos_mixer_pro/core/state/global_state.dart';" not in content:
        content = content.replace(
            "import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';",
            "import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';\nimport 'package:atmos_mixer_pro/core/state/global_state.dart';"
        )

    # In build method of _SpeakerInspectorPanelState, we need to read outputChannelCount and all speakers
    old_build_start = """  @override
  Widget build(BuildContext context) {
    final speakers = ref.watch(speakerLayoutProvider);
    final speaker = speakers.where((s) => s.id == widget.speakerId).firstOrNull;

    if (speaker == null) return const SizedBox.shrink();

    final rooms = ref.watch(roomZoneProvider);
    double roomW = 10.0;
    double roomD = 10.0;
    double roomH = 3.5;
    if (speaker.roomId != null) {"""

    new_build_start = """  @override
  Widget build(BuildContext context) {
    final speakers = ref.watch(speakerLayoutProvider);
    final speaker = speakers.where((s) => s.id == widget.speakerId).firstOrNull;
    final engineState = ref.watch(engineStateProvider);
    final maxChannels = engineState.outputChannelCount;

    if (speaker == null) return const SizedBox.shrink();

    final rooms = ref.watch(roomZoneProvider);
    double roomW = 10.0;
    double roomD = 10.0;
    double roomH = 3.5;
    if (speaker.roomId != null) {"""

    content = content.replace(old_build_start, new_build_start)
    
    # Update the Dropdown to dynamically load channels up to maxChannels
    old_dropdown = """                  child: DropdownButtonHideUnderline(
                    child: DropdownButton<int>(
                      value: speaker.channel,
                      dropdownColor: const Color(0xFF1E2632),
                      style: const TextStyle(color: Colors.white, fontSize: 14, fontWeight: FontWeight.bold),
                      items: [
                        ...List.generate(64, (i) => DropdownMenuItem(value: i, child: Text('Output CH ${i + 1}'))),
                        if (speaker.channel >= 64) DropdownMenuItem(value: speaker.channel, child: Text('Output CH ${speaker.channel + 1}')),
                      ],
                      onChanged: (val) {
                        if (val != null) {
                          ref.read(speakerLayoutProvider.notifier).updateSpeaker(speaker!.copyWith(channel: val));
                        }
                      },
                    ),
                  ),"""

    new_dropdown = """                  child: DropdownButtonHideUnderline(
                    child: DropdownButton<int>(
                      value: speaker.channel < maxChannels ? speaker.channel : (maxChannels > 0 ? 0 : speaker.channel),
                      dropdownColor: const Color(0xFF1E2632),
                      style: const TextStyle(color: Colors.white, fontSize: 14, fontWeight: FontWeight.bold),
                      items: List.generate(maxChannels > 0 ? maxChannels : 2, (i) {
                        final inUseBy = speakers.where((s) => s.channel == i && s.id != speaker.id).firstOrNull;
                        final label = inUseBy != null ? 'Output CH ${i + 1} (In Use: ${inUseBy.id.substring(0, Math.min(3, inUseBy.id.length))})' : 'Output CH ${i + 1}';
                        return DropdownMenuItem(
                          value: i, 
                          child: Row(
                            children: [
                              Text(
                                label, 
                                style: TextStyle(color: inUseBy != null ? Colors.white54 : Colors.white)
                              ),
                              if (speaker.channel == i) const Padding(
                                padding: EdgeInsets.only(left: 8.0),
                                child: Icon(Icons.check, size: 16, color: Colors.lightBlueAccent),
                              )
                            ]
                          )
                        );
                      }),
                      onChanged: (val) {
                        if (val != null) {
                          ref.read(speakerLayoutProvider.notifier).updateSpeaker(speaker!.copyWith(channel: val));
                        }
                      },
                    ),
                  ),"""

    content = content.replace(old_dropdown, new_dropdown)
    
    # We also need to import dart:math as Math for substring length
    if "import 'dart:math' as Math;" not in content:
        content = content.replace(
            "import 'package:flutter/material.dart';",
            "import 'package:flutter/material.dart';\nimport 'dart:math' as Math;"
        )

    with open(path, "w") as f:
        f.write(content)

main()
