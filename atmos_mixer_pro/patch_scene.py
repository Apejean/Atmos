import re

with open('rust/src/api/scene.rs', 'r') as f:
    content = f.read()

content = re.sub(r'AtmosError::IoError\(([^)]+)\)', r'AtmosError { message: \1 }', content)
content = re.sub(r'AtmosError::SerializationError\(([^)]+)\)', r'AtmosError { message: \1 }', content)

with open('rust/src/api/scene.rs', 'w') as f:
    f.write(content)
