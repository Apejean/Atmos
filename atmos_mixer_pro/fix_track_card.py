import re

with open('lib/features/dashboard/widgets/track_card.dart', 'r') as f:
    content = f.read()

# find where it ends
start = content.rfind('  }\n"""')
if start == -1:
    # my script didn't write it that way. 
    pass

# let's just append the missing `}` and move typedef outside
