import re

with open('lib/features/dashboard/widgets/object_panner_modal.dart', 'r') as f:
    content = f.read()

content = content.replace("withOpacity", "withValues(alpha: ")
# we need to close the bracket... this is tricky with regex. Let's do it with python properly

content = re.sub(r'\.withOpacity\(([^)]+)\)', r'.withValues(alpha: \1)', content)

with open('lib/features/dashboard/widgets/object_panner_modal.dart', 'w') as f:
    f.write(content)
