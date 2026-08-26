import re

with open("rust/src/common/commands.rs", "r") as f:
    content = f.read()

content = content.replace(
    "eq_bands: Vec<crate::common::config::EqBand>,\n        phase_invert: bool,\n        gain_db: f32,\n    },",
    "eq_bands: Vec<crate::common::config::EqBand>,\n    },"
)

with open("rust/src/common/commands.rs", "w") as f:
    f.write(content)

with open("rust/src/audio/engine.rs", "r") as f:
    content = f.read()

content = content.replace(
    "AudioCommand::ApplyChannelTuning { channel, delay_ms, eq_bands, phase_invert, gain_db } => {",
    "AudioCommand::ApplyChannelTuning { channel, delay_ms, eq_bands } => {"
)
content = content.replace(
    "for (channel, delay_ms, eq_bands, phase_invert, gain_db) in tunings {",
    "for (channel, delay_ms, eq_bands) in tunings {"
)
content = re.sub(r"mixer\.channel_dsp\[channel\]\.is_phase_inverted = phase_invert;\n\s*mixer\.channel_dsp\[channel\]\.target_gain_db = gain_db;\n", "", content)

with open("rust/src/audio/engine.rs", "w") as f:
    f.write(content)

