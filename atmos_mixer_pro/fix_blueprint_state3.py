import re

with open('lib/features/exhibition/state/blueprint_state.dart', 'r') as f:
    content = f.read()

# Replace prefs.setString etc with nothing to just keep it compiled
content = re.sub(r'prefs\.set[A-Za-z]+\([^;]+\);', '', content)

with open('lib/features/exhibition/state/blueprint_state.dart', 'w') as f:
    f.write(content)
print("Removed prefs set calls")
