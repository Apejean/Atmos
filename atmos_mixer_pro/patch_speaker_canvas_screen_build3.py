import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Make 3D speakers clickable so we can see the Inspector panel since the 2D view is a placeholder
content = re.sub(
    r"(child: Text\(\n\s*'Ch\$\{node\.channel\}',\n\s*style: const TextStyle\(color: Colors\.white, fontSize: 8, fontWeight: FontWeight\.bold\),\n\s*\),)",
    r"child: GestureDetector(\n                            onTap: () => ref.read(selectedSpeakerProvider.notifier).state = node.id,\n                            child: Text(\n                              'Ch${node.channel}',\n                              style: const TextStyle(color: Colors.white, fontSize: 8, fontWeight: FontWeight.bold),\n                            ),\n                          ),",
    content
)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)

with open('lib/features/exhibition/widgets/viewport_3d/room_3d_viewport.dart', 'r') as f:
    content2 = f.read()

content2 = re.sub(
    r"(child: Text\(\n\s*'Ch\$\{node\.channel\}',\n\s*style: const TextStyle\(color: Colors\.white, fontSize: 8, fontWeight: FontWeight\.bold\),\n\s*\),)",
    r"child: GestureDetector(\n                            onTap: () => ref.read(selectedSpeakerProvider.notifier).state = node.id,\n                            child: Text(\n                              'Ch${node.channel}',\n                              style: const TextStyle(color: Colors.white, fontSize: 8, fontWeight: FontWeight.bold),\n                            ),\n                          ),",
    content2
)
with open('lib/features/exhibition/widgets/viewport_3d/room_3d_viewport.dart', 'w') as f:
    f.write(content2)
