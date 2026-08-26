import re

with open('lib/features/dashboard/widgets/track_card.dart', 'r') as f:
    content = f.read()

content = content.replace("const TrajectorySettingsModal()", "TrajectorySettingsModal(trackId: widget.track.id)")

with open('lib/features/dashboard/widgets/track_card.dart', 'w') as f:
    f.write(content)
