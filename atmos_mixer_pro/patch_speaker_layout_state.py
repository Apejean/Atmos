import re

# Looks like speakerLayoutProvider returns a List<SpeakerNode> directly, not a class with `nodes`.
# And its notifier method is probably `updateSpeaker` not `updateNode`.

with open('lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart', 'r') as f:
    inspector_content = f.read()

inspector_content = inspector_content.replace(
    'layout.nodes.firstWhere',
    'layout.firstWhere'
)
inspector_content = inspector_content.replace(
    'ref.read(speakerLayoutProvider.notifier).updateNode(updated);',
    'ref.read(speakerLayoutProvider.notifier).updateSpeaker(updated);'
)

with open('lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart', 'w') as f:
    f.write(inspector_content)

with open('lib/features/exhibition/widgets/viewport_3d/room_3d_viewport.dart', 'r') as f:
    viewport_content = f.read()

viewport_content = viewport_content.replace(
    'layout.nodes',
    'layout'
)

with open('lib/features/exhibition/widgets/viewport_3d/room_3d_viewport.dart', 'w') as f:
    f.write(viewport_content)

