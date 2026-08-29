with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

old_logic = """                final nextChannel = speakers.isEmpty
                    ? 0
                    : (speakers.map((s) => s.channel).reduce((a, b) => a > b ? a : b) + 1);"""

new_logic = """                // Find the lowest available empty channel
                int nextChannel = 0;
                final usedChannels = speakers.map((s) => s.channel).toSet();
                while (usedChannels.contains(nextChannel)) {
                  nextChannel++;
                }"""

content = content.replace(old_logic, new_logic)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
