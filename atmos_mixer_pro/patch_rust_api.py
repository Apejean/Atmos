import re

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'r') as f:
    content = f.read()

content = content.replace('rust_api.SpeakerPhysicalSpec(', 'SpeakerPhysicalSpec(')

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'w') as f:
    f.write(content)
