import re

files = [
    'test/dashboard_features_test.dart',
    'test/master_verification_items_7_to_20_test.dart',
    'lib/features/settings/widgets/preferences_modal.dart',
    'lib/features/dashboard/screens/dashboard_screen.dart',
    'lib/features/dashboard/widgets/room_card.dart'
]

for file in files:
    with open(file, 'r') as f:
        content = f.read()

    # For tests, we use const []
    if file.startswith('test'):
        content = re.sub(r'AppConfig\(', r'AppConfig(\n          oscWhitelist: const [],', content)
    else:
        # In UI files, find instances like: 
        # _tempConfig = AppConfig(
        # updated = AppConfig(
        # AppConfig( ... ) -> if no assignment, just use const [] or currentConfig.oscWhitelist
        
        def replacer(match):
            prefix = match.group(1)
            var_name = match.group(2)
            
            if var_name:
                # If there's an assignment like `_tempConfig = AppConfig(` or `updated = AppConfig(`
                # The config to copy from is usually the same variable or `widget.config`
                if var_name == '_tempConfig':
                    return prefix + '\n          oscWhitelist: _tempConfig.oscWhitelist,'
                elif var_name == 'updated':
                    return prefix + '\n          oscWhitelist: config.oscWhitelist,' # wait, let's use const [] if unsure
                else:
                    return prefix + '\n          oscWhitelist: const [],'
            else:
                return prefix + '\n          oscWhitelist: const [],'

        # This regex matches an optional assignment before `AppConfig(`
        # e.g., `_tempConfig = AppConfig(`
        # Group 1: The whole match, Group 2: The variable name
        content = re.sub(r'((?:(\w+)\s*=\s*)?AppConfig\()', replacer, content)

    with open(file, 'w') as f:
        f.write(content)

