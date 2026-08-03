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
        content = f.read()

    # Find all AppConfig( blocks and add oscWhitelist if missing
    # We will do this by finding AppConfig( and finding the matching closing parenthesis, but that's hard with regex.
    # Instead, we just look for AppConfig( and insert right after it. But if the next few lines have oscWhitelist, we skip.
    
    parts = content.split('AppConfig(')
    new_content = parts[0]
    
    for i in range(1, len(parts)):
        part = parts[i]
        prefix = 'AppConfig('
        
        # Look backwards in new_content to see if there was an assignment like `var_name = `
        var_match = re.search(r'(\w+)\s*=\s*$', new_content)
        var_name = var_match.group(1) if var_match else None
        
        # Determine what to insert
        insert_str = ''
        if file.startswith('test'):
            insert_str = '\n          oscWhitelist: const [],'
        else:
            if var_name == '_tempConfig':
                insert_str = '\n          oscWhitelist: _tempConfig.oscWhitelist,'
            elif var_name == 'updated':
                insert_str = '\n          oscWhitelist: config.oscWhitelist,'
            else:
                insert_str = '\n          oscWhitelist: const [],'
                
        # Check if oscWhitelist is already there in the next 1000 characters
        next_chunk = part[:1000]
        if 'oscWhitelist:' not in next_chunk:
            new_content += prefix + insert_str + part
        else:
            new_content += prefix + part

    with open(file, 'w') as f:
        f.write(new_content)

