import re

with open("rust/src/audio/mixer.rs", "r") as f:
    content = f.read()

content = content.replace("pub soloed_channels: Vec<usize>,", "pub any_soloed: bool,")
content = content.replace("soloed_channels: Vec::new(),", "any_soloed: false,")
content = content.replace("if !self.soloed_channels.is_empty() && !self.soloed_channels.contains(&ch) {", "if self.any_soloed && !self.channel_dsp[ch].is_soloed {")

with open("rust/src/audio/mixer.rs", "w") as f:
    f.write(content)

with open("rust/src/audio/engine.rs", "r") as f:
    content = f.read()

content = content.replace("let mut new_solos = Vec::new();\n", "let mut any_soloed = false;\n")
content = content.replace("new_solos.push(i);\n", "any_soloed = true;\n")
content = content.replace("mixer.soloed_channels = new_solos;", "mixer.any_soloed = any_soloed;")

with open("rust/src/audio/engine.rs", "w") as f:
    f.write(content)
