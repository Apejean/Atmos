import re

with open('lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart', 'r') as f:
    inspector_content = f.read()

# Fix 1: blueprint.speakers -> ref.watch(speakerLayoutProvider).speakers
# Wait, let's see how speakerLayoutProvider is exposed. 
# Usually, speakerLayoutProvider returns SpeakerLayoutData, which has `nodes`.
inspector_content = inspector_content.replace(
    'import \'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart\';',
    'import \'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart\';\nimport \'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart\';'
)
inspector_content = inspector_content.replace(
    'final blueprint = ref.watch(blueprintProvider);',
    'final layout = ref.watch(speakerLayoutProvider);'
)
inspector_content = inspector_content.replace(
    'blueprint.speakers.firstWhere',
    'layout.nodes.firstWhere'
)
inspector_content = inspector_content.replace(
    'speaker.channelIndex',
    'speaker.channel'
)
inspector_content = inspector_content.replace(
    'speaker.label',
    'speaker.channel.toString()'
)
inspector_content = inspector_content.replace(
    'ref.read(blueprintProvider.notifier).updateSpeaker(updated);',
    'ref.read(speakerLayoutProvider.notifier).updateNode(updated);'
)

with open('lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart', 'w') as f:
    f.write(inspector_content)

# Fix 2: room_3d_viewport.dart
with open('lib/features/exhibition/widgets/viewport_3d/room_3d_viewport.dart', 'r') as f:
    viewport_content = f.read()

viewport_content = viewport_content.replace(
    'import \'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart\';',
    'import \'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart\';\nimport \'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart\';\nimport \'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart\';'
)
viewport_content = viewport_content.replace(
    'final blueprint = ref.watch(blueprintProvider);',
    'final layout = ref.watch(speakerLayoutProvider);'
)
viewport_content = viewport_content.replace(
    'blueprint.speakers',
    'layout.nodes'
)
viewport_content = viewport_content.replace(
    'spk.channelIndex',
    'spk.channel'
)

with open('lib/features/exhibition/widgets/viewport_3d/room_3d_viewport.dart', 'w') as f:
    f.write(viewport_content)

