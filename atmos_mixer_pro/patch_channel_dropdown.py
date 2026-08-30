import re

def main():
    path = "lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart"
    with open(path, "r") as f:
        content = f.read()

    old_dropdown = """                  child: DropdownButtonHideUnderline(
                    child: DropdownButton<int>(
                      value: speaker.channel,
                      dropdownColor: const Color(0xFF1E2632),
                      style: const TextStyle(color: Colors.white, fontSize: 14, fontWeight: FontWeight.bold),
                      items: List.generate(24, (i) => DropdownMenuItem(value: i, child: Text('Selected Speaker: CH ${i + 1}'))),
                      onChanged: (val) {
                        if (val != null) {
                          ref.read(speakerLayoutProvider.notifier).updateSpeaker(speaker!.copyWith(channel: val));
                        }
                      },
                    ),
                  ),"""

    new_dropdown = """                  child: DropdownButtonHideUnderline(
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

    content = content.replace(old_dropdown, new_dropdown)

    with open(path, "w") as f:
        f.write(content)

main()
