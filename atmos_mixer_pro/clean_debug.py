import os

path = 'assets/3d_simulator/studio_engine.html'
with open(path, 'r') as f:
    content = f.read()

# Remove the debug logs
import re
content = re.sub(r'// Throttled logging for debugging.*?if \(isHit\)', 'if (isHit)', content, flags=re.DOTALL)

with open(path, 'w') as f:
    f.write(content)

print("Debug logs cleaned.")
