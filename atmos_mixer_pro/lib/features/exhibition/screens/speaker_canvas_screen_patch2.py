import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Fix State access (we just overrode the file so we know exactly what's there)
content = content.replace(
    'final selectedSpeakerId = ref.watch(selectedSpeakerIdProvider);',
    'final selectedSpeakerId = ref.watch(selectedSpeakerProvider);'
)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
