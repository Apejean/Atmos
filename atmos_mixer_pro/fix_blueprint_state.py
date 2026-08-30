import re

with open('lib/features/exhibition/state/blueprint_state.dart', 'r') as f:
    content = f.read()

# Add missing import for SharedPreferences
import_str = "import 'package:shared_preferences/shared_preferences.dart';"
if import_str not in content:
    content = import_str + "\n" + content

with open('lib/features/exhibition/state/blueprint_state.dart', 'w') as f:
    f.write(content)

print("Fixed blueprint_state.dart")
