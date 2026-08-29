import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# Let's adjust the multiplier from 1.25 to 1.8. The other agent set it to 1.25, but the user says it's too tight still.
# Wait, let me check the codebase right now to see what's actually there.
