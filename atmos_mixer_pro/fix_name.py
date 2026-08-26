import re

with open('lib/features/dashboard/widgets/room_calibration_wizard_modal.dart', 'r') as f:
    content = f.read()

# label instead of name
content = content.replace("child: Text(r.name)", "child: Text(r.label)")

with open('lib/features/dashboard/widgets/room_calibration_wizard_modal.dart', 'w') as f:
    f.write(content)
