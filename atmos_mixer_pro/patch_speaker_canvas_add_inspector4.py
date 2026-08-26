import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# I see `_DraggableSpeakerWidget`. I need to add an onTap handler to it to select the speaker!
# Let's check `_DraggableSpeakerWidget` definition.
import os
os.system('sed -n "2280,2300p" lib/features/exhibition/screens/speaker_canvas_screen.dart')

