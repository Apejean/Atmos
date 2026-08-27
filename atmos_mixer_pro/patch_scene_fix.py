import re

with open('rust/src/api/scene.rs', 'r') as f:
    content = f.read()

content = content.replace("AtmosError { message: e.to_string( })", "AtmosError { message: e.to_string() }")

with open('rust/src/api/scene.rs', 'w') as f:
    f.write(content)
