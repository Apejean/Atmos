import os
import re

def process_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()
    
    if "RoomZone(" in content and "transmissionLossDb:" not in content:
        content = content.replace("RoomZone(", "RoomZone(transmissionLossDb: 0.0, ")
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

