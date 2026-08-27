import os
import re

def process_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()
    
    if "RoomZone(transmissionLossDb: 0.0, " in content:
        content = content.replace("RoomZone(transmissionLossDb: 0.0, ", "RoomZone(")
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

