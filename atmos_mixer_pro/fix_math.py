import re

with open('lib/features/dashboard/widgets/object_panner_modal.dart', 'r') as f:
    content = f.read()

content = content.replace("import 'dart:math' as math;\n", "")

with open('lib/features/dashboard/widgets/object_panner_modal.dart', 'w') as f:
    f.write(content)
