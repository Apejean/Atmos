import re

with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'r') as f:
    content = f.read()

# Fix RenderFlex overflow in Stepper Input Box
# 1. Stepper icons are too big or container height is too small
# 2. Main content overflowing to the right in Row

content = content.replace('height: 28,', 'height: 32,')
content = content.replace('size: 14', 'size: 12')
content = content.replace('width: 20,', 'width: 24,')
content = content.replace('width: 280,', 'width: 260,')

with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'w') as f:
    f.write(content)

