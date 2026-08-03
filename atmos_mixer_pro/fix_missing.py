import re
import os
import glob

def fix_file(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    # Find AppConfig( with no oscWhitelist inside it
    # Because there are newlines, we can use a regex that matches AppConfig( ... ) and checks if oscWhitelist is inside
    
    parts = content.split('AppConfig(')
    new_content = parts[0]
    for part in parts[1:]:
        # Find the matching closing parenthesis
        level = 1
        inner_idx = 0
        while inner_idx < len(part) and level > 0:
            if part[inner_idx] == '(':
                level += 1
            elif part[inner_idx] == ')':
                level -= 1
            inner_idx += 1
            
        inner_content = part[:inner_idx-1]
        rest = part[inner_idx-1:]
        
        if 'oscWhitelist' not in inner_content:
            new_content += 'AppConfig(oscWhitelist: const [], ' + inner_content + rest
        else:
            new_content += 'AppConfig(' + inner_content + rest
            
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(new_content)

for f in glob.glob('lib/**/*.dart', recursive=True) + glob.glob('test/**/*.dart', recursive=True):
    # Only process if AppConfig is instantiated
    if 'frb_generated.dart' in f or 'config.dart' in f or 'config.freezed.dart' in f:
        continue
    with open(f, 'r', encoding='utf-8') as file:
        if 'AppConfig(' in file.read():
            fix_file(f)
print("Done fixing missing")
