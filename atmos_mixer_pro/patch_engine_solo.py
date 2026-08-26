import re

with open("rust/src/audio/engine.rs", "r") as f:
    content = f.read()

content = re.sub(r"let mut new_solos = Vec::new\(\);\n", "let mut any_soloed = false;\n", content)
content = re.sub(r"new_solos\.push\(i\);\n", "any_soloed = true;\n", content)
content = re.sub(r"mixer\.soloed_channels = new_solos;\n", "mixer.any_soloed = any_soloed;\n", content)

with open("rust/src/audio/engine.rs", "w") as f:
    f.write(content)
