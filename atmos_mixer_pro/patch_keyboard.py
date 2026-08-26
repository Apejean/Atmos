import re
path = "lib/features/dashboard/screens/dashboard_screen.dart"
with open(path, "r") as f:
    content = f.read()

# Add missing import for LogicalKeyboardKey
if "package:flutter/services.dart" not in content:
    content = content.replace("import 'package:flutter/material.dart';", "import 'package:flutter/material.dart';\nimport 'package:flutter/services.dart';")

with open(path, "w") as f:
    f.write(content)
