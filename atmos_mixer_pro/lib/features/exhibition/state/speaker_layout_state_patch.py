import re

with open('lib/features/exhibition/state/speaker_layout_state.dart', 'r') as f:
    content = f.read()

# Add selected speaker provider if not exists
if 'selectedSpeakerProvider' not in content:
    content += '''
final selectedSpeakerProvider = StateProvider<String?>((ref) => null);

// Also add a helper to SpeakerLayoutState to select
// But StateProvider is easier.
'''

with open('lib/features/exhibition/state/speaker_layout_state.dart', 'w') as f:
    f.write(content)
