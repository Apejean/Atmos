import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Fix double declaration of _showHeatmap and _targetSPL
content = re.sub(r'bool _showHeatmap = true;\n', '', content)
content = re.sub(r'double _targetSPL = 75.0;\n\n  double _targetSPL = 75.0;', 'double _targetSPL = 75.0;', content)
content = re.sub(r'bool _showHeatmap = false;\n  bool _showHeatmap = false;', 'bool _showHeatmap = false;', content)
content = content.replace('double _targetSPL = 75.0;\n  bool _showHeatmap = false;\n\n\n  bool _showHeatmap = true;', 'double _targetSPL = 75.0;\n  bool _showHeatmap = false;')

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
