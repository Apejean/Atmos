import re

# Remove onClose
with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

content = content.replace("                              onClose: () => setState(() => _inspectorSpeakerId = null),\n", "")

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)

# Add import to dashboard
with open('lib/features/dashboard/screens/dashboard_screen.dart', 'r') as f:
    content = f.read()

import_stmt = "import 'package:atmos_mixer_pro/src/rust/api/calibration.dart';"
if import_stmt not in content:
    content = content.replace("import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;", 
                              "import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;\n" + import_stmt)

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'w') as f:
    f.write(content)
