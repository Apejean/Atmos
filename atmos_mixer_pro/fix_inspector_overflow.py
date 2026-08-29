import re

with open('lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart', 'r') as f:
    content = f.read()

pattern = r"Text\(label, style: const TextStyle\(color: Colors\.white70, fontSize: 12\)\),"
replacement = """Expanded(child: Text(label, style: const TextStyle(color: Colors.white70, fontSize: 12), overflow: TextOverflow.ellipsis)),"""

content = re.sub(pattern, replacement, content)

with open('lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart', 'w') as f:
    f.write(content)

