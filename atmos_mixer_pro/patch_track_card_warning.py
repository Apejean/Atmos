import re

with open("lib/features/dashboard/widgets/track_card.dart", "r") as f:
    content = f.read()

# Remove unused import and variable
content = re.sub(r"import 'dart:math' as math;\n", "", content)
content = re.sub(r"\s*final config = ref\.watch\(audioConfigProvider\);", "", content)

with open("lib/features/dashboard/widgets/track_card.dart", "w") as f:
    f.write(content)
