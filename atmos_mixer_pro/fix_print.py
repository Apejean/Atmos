with open('lib/features/dashboard/state/output_routing_state.dart', 'r') as f:
    content = f.read()

content = content.replace("import 'dart:convert';", "import 'dart:convert';\nimport 'package:flutter/foundation.dart';")
content = content.replace("print(", "debugPrint(")

with open('lib/features/dashboard/state/output_routing_state.dart', 'w') as f:
    f.write(content)
