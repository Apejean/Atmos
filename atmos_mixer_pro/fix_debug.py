import os

path = 'assets/3d_simulator/studio_engine.html'
with open(path, 'r') as f:
    content = f.read()

content = content.replace("SPK ${speakerData?.index}", "SPK ${spk.id}")

with open(path, 'w') as f:
    f.write(content)

print("Fixed debug log.")
