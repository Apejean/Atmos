with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# The replace failed to match exactly, probably because I missed some parts in the end block
# Let's fix the end block manually

# Let's just restore from backup or patch specifically
# I will use a simple regex to fix the end of ModelViewer

import re

# find ModelViewer
start_idx = content.find('ModelViewer(')
if start_idx != -1:
    # count parentheses
    open_parens = 0
    end_idx = -1
    for i in range(start_idx + 11, len(content)):
        if content[i] == '(':
            open_parens += 1
        elif content[i] == ')':
            if open_parens == 1:
                end_idx = i
                break
            open_parens -= 1
            
    if end_idx != -1:
        # Check what follows
        # We want to replace the closing part if we added a GestureDetector
        if 'GestureDetector' in content:
            # We already have GestureDetector
            # Just ensure the closing parentheses match
            pass

# Wait, let's look at the error again. 
# error - dynamic_3d_room.dart:160:11 - Expected an identifier. - missing_identifier
# line 160 is:           if (widget.showHeatmap)
# wait, line 152:
# 151	              interactionPrompt: InteractionPrompt.none,
# 152	
# 153	              
# 154	
# 155	              
# 156	            ),
# 157	          ),
#
# Ah, the replace removed some arguments!
# old_mv_end was:
#               maxCameraOrbit: 'auto auto 2000m',
#               minCameraOrbit: 'auto auto 1.5m',
#               exposure: 1.1,
#               shadowIntensity: 0.6,
#               shadowSoftness: 0.8,
#             ),
#           ),
# But they were removed! Wait, no, they were not removed, they were replaced.
# Let's check what happened.

