import re

with open('lib/features/dashboard/widgets/object_panner_modal.dart', 'r') as f:
    content = f.read()

# Logic Pro Y Axis: 0.0 is Back, 1.0 is Front.
# "Center-Front, X=0, Y=1.0" in Logic Pro's -1 to +1 coordinate space means Y=+1.0 is Front, Y=-1.0 is Back.
# In our mapped 0.0 to 1.0 space: 
# Back/Front: -1.000 (Back) ~ +1.000 (Front)
# If val is 0.0, mapped is -1.0 (Back)
# If val is 1.0, mapped is +1.0 (Front)
# But wait, in the UI Front is usually at the top, Back is at the bottom.
# So _y = 0.0 (top of minimap) should mean Front (+1.0).
# _y = 1.0 (bottom of minimap) should mean Back (-1.0).
# The readout logic should be: mapped = ( (1.0 - _y) - 0.5 ) * 2.0;

content = content.replace("_buildReadout('Back/Front', _formatVal(1.0 - _y)), // Inverse Y for Back/Front (0 is front, 1 is back)",
"_buildReadout('Back/Front', _formatVal(1.0 - _y)),")

with open('lib/features/dashboard/widgets/object_panner_modal.dart', 'w') as f:
    f.write(content)
