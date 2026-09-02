import os

path = 'assets/3d_simulator/studio_engine.html'
with open(path, 'r') as f:
    content = f.read()

content = content.replace("speakerMeshes.forEach(spk => {", "speakersGroup.children.forEach(spk => {")

with open(path, 'w') as f:
    f.write(content)
print("Variable fixed.")
