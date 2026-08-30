# The html might be missing the transparent background? 
# Wait, the error is `Failed to load resource: net::ERR_CONNECTION_REFUSED` or something?
# No, let's check `_startLocalServer`. Is it actually starting?
# Wait, let's check what `_startLocalServer()` does.
with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()
import re
print(re.search(r'_startLocalServer.*?try \{', content, re.DOTALL).group(0))
