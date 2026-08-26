import re

with open('lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart', 'r') as f:
    content = f.read()

# I see a compilation error might still happen if speaker.channel doesn't exist.
# Let's check `SpeakerNode` fields.
import os
os.system('cat lib/features/exhibition/models/speaker_node.dart | grep "channel"')

