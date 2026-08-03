import re
import os

files = [
    'test/dashboard_features_test.dart',
    'test/master_verification_items_7_to_20_test.dart',
    'lib/features/settings/widgets/preferences_modal.dart',
    'lib/features/dashboard/screens/dashboard_screen.dart',
    'lib/features/dashboard/widgets/room_card.dart'
]

for file in files:
    if not os.path.exists(file):
        continue
    with open(file, 'r') as f:
        lines = f.readlines()
        
    # We want to remove duplicate consecutive 'oscWhitelist' lines or within the same AppConfig block
    # Simple approach: keep track of braces. Inside an AppConfig(, only keep the first oscWhitelist
    
    new_lines = []
    in_appconfig = False
    appconfig_depth = 0
    seen_whitelist = False
    
    for line in lines:
        if 'AppConfig(' in line:
            in_appconfig = True
            seen_whitelist = False
            # We don't handle multiple AppConfig on the same line, assuming standard formatting
        
        if in_appconfig:
            if 'oscWhitelist:' in line:
                if seen_whitelist:
                    continue # skip duplicate
                else:
                    seen_whitelist = True
            
            # Simple bracket counting to exit block (not perfect but works for dart formatting)
            if ')' in line and not '(' in line.split(')')[0]:
                 # Just approximate, if we see `),` or `)` we might be closing
                 # Actually, we can just reset seen_whitelist when we see `AppConfig(`
                 pass
                 
        new_lines.append(line)
        
    with open(file, 'w') as f:
        f.writelines(new_lines)

