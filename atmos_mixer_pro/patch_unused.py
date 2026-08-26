with open("lib/features/dashboard/screens/dashboard_screen.dart", "r") as f:
    content = f.read()

content = content.replace("import 'dart:math' as math;\n", "")

with open("lib/features/dashboard/screens/dashboard_screen.dart", "w") as f:
    f.write(content)
