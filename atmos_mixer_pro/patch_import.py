import re

with open('lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart', 'r') as f:
    content = f.read()

content = content.replace("import 'package:flutter_riverpod/flutter_riverpod.dart';", "import 'package:flutter_riverpod/flutter_riverpod.dart';\nimport 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;")

with open('lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart', 'w') as f:
    f.write(content)
