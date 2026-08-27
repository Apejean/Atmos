import os
import re

def process_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    # Find all occurrences of AppConfig( ... ) and add globalReverbMix: 0.0, globalReverbDecay: 1.0, if not present.
    # It might span multiple lines, so we can replace 'AppConfig(' with a custom function, but since it's just named arguments, we can just insert them right after 'AppConfig('
    # Wait, inserting right after 'AppConfig(' is safest.
    
    if "AppConfig(" in content and "globalReverbMix:" not in content:
        # We need to make sure we don't mess up other things.
        # Just replace "AppConfig(" with "AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0,"
        content = content.replace("AppConfig(", "AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, ")
        
        with open(filepath, 'w') as f:
            f.write(content)

for root, _, files in os.walk("lib"):
    for file in files:
        if file.endswith(".dart"):
            process_file(os.path.join(root, file))

for root, _, files in os.walk("test"):
    for file in files:
        if file.endswith(".dart"):
            process_file(os.path.join(root, file))

