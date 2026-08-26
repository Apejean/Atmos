import re

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'r') as f:
    content = f.read()

# Add import dart:typed_data if not there
if "import 'dart:typed_data';" not in content:
    content = content.replace("import 'package:flutter/material.dart';", "import 'package:flutter/material.dart';\nimport 'dart:typed_data';")

# fix apiCalculate3DCalibration to rust_api.apiCalculate3DCalibration
content = content.replace("apiCalculate3DCalibration(", "rust_api.apiCalculate3DCalibration(")

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'w') as f:
    f.write(content)
