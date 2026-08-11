import re

with open('lib/features/settings/widgets/preferences_modal.dart', 'r', encoding='utf-8') as f:
    content = f.read()

# Replace clearOscAddress: XXX.clearOscAddress, with clearOscAddress: XXX.clearOscAddress, volumeOscAddress: XXX.volumeOscAddress,
# We'll use a regex that captures the variable prefix (like r, room, tempRoom)
content = re.sub(
    r'(clearOscAddress:\s*([a-zA-Z0-9_]+)\.clearOscAddress,)',
    r'\1\n              volumeOscAddress: \2.volumeOscAddress,',
    content
)

# And for the one where clearOscAddress: val is used, which is around line 1661:
# We need to add volumeOscAddress: room.volumeOscAddress, after it
content = re.sub(
    r'(clearOscAddress:\s*val,)',
    r'\1\n                            volumeOscAddress: room.volumeOscAddress,',
    content
)

with open('lib/features/settings/widgets/preferences_modal.dart', 'w', encoding='utf-8') as f:
    f.write(content)
