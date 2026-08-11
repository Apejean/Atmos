import re

files_to_fix = [
    'lib/features/dashboard/screens/dashboard_screen.dart',
    'lib/features/dashboard/widgets/room_card.dart'
]

for filepath in files_to_fix:
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    # The issue is we are copying an existing RoomConfig.
    # Find clearOscAddress: XXX.clearOscAddress, and insert volumeOscAddress: XXX.volumeOscAddress,
    # Also find clearOscAddress: room.clearOscAddress, etc.
    # What if clearOscAddress: val? Let's check manually if there are any.
    
    # We will use a safe regex that captures the expression before .clearOscAddress
    content = re.sub(
        r'(clearOscAddress:\s*([a-zA-Z0-9_\.]+)\.clearOscAddress,)',
        r'\1\n                                volumeOscAddress: \2.volumeOscAddress,',
        content
    )
    
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)

