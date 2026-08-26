import re

with open('lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart', 'r') as f:
    content = f.read()

# Fix selectSpeaker
content = content.replace(
    'ref.read(speakerLayoutProvider.notifier).selectSpeaker(null)',
    'ref.read(selectedSpeakerProvider.notifier).state = null'
)

with open('lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart', 'w') as f:
    f.write(content)
