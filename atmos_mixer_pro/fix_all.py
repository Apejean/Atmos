import re
import os
import glob

def fix_file(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    # Find all AppConfig(...) blocks
    # We need to handle nested parenthesis, so a simple regex might not work, but we can do it iteratively
    
    # Actually, a much easier way:
    # 1. Remove all lines containing `oscWhitelist:` (we'll re-add them)
    # 2. Add them back appropriately
    
    lines = content.split('\n')
    new_lines = []
    
    i = 0
    while i < len(lines):
        line = lines[i]
        
        # Remove duplicate oscWhitelist lines completely if they are alone on the line
        if 'oscWhitelist:' in line:
            # We'll skip this line, meaning it's deleted. 
            # But wait, what if it's on the same line as AppConfig(?
            if line.strip().startswith('oscWhitelist:') or 'oscWhitelist:' in line:
                # If there's other code, we can't just drop the line.
                # Let's replace 'oscWhitelist: [^,]+,' with ''
                line = re.sub(r'oscWhitelist:\s*[^,]+,?\s*', '', line)
                if not line.strip() and not lines[i].strip() == '':
                    i += 1
                    continue # drop line if it became empty
        
        new_lines.append(line)
        i += 1
        
    # Now we need to add `oscWhitelist: config.oscWhitelist` or `[]` to all AppConfig(
    content = '\n'.join(new_lines)
    
    # Now, find all `AppConfig(` and add `oscWhitelist: const [],` or `config.oscWhitelist`
    # Let's just do it manually with regex.
    # We replace `AppConfig(` with `AppConfig(oscWhitelist: const [],`
    # But for files where config is available, we want `config.oscWhitelist`
    
    if 'config.oscWhitelist' in filepath or 'dashboard_screen' in filepath or 'preferences_modal' in filepath:
        # Actually, let's just look at the variable name used for oscPort
        def replacer(match):
            inner = match.group(1)
            # Find what is used for oscPort
            m = re.search(r'oscPort:\s*([a-zA-Z0-9_]+)\.oscPort', inner)
            if m:
                var_name = m.group(1)
                return f"AppConfig(\noscWhitelist: {var_name}.oscWhitelist,\n{inner})"
            elif 'oscPort: int.tryParse' in inner:
                return f"AppConfig(\noscWhitelist: _tempConfig.oscWhitelist,\n{inner})"
            else:
                return f"AppConfig(\noscWhitelist: const [],\n{inner})"
                
        content = re.sub(r'AppConfig\(([\s\S]*?)\)', replacer, content)
    
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)

for f in glob.glob('lib/**/*.dart', recursive=True) + glob.glob('test/**/*.dart', recursive=True):
    # Only process if AppConfig is instantiated
    with open(f, 'r', encoding='utf-8') as file:
        if 'AppConfig(' in file.read():
            fix_file(f)

print("Done fixing AppConfig")
