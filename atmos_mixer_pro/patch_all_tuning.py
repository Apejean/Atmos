import re

# 1. Update commands.rs
with open("rust/src/common/commands.rs", "r") as f:
    lines = f.readlines()

out = []
for line in lines:
    out.append(line)
    if "eq_bands: Vec<crate::common::config::EqBand>," in line:
        indent = line[:len(line) - len(line.lstrip())]
        out.append(f"{indent}phase_invert: bool,\n")
        out.append(f"{indent}gain_db: f32,\n")

with open("rust/src/common/commands.rs", "w") as f:
    f.writelines(out)

# 2. Update engine.rs
with open("rust/src/audio/engine.rs", "r") as f:
    lines = f.readlines()

out = []
for line in lines:
    if "AudioCommand::ApplyChannelTuning { channel, delay_ms, eq_bands } => {" in line:
        out.append("                AudioCommand::ApplyChannelTuning { channel, delay_ms, eq_bands, phase_invert, gain_db } => {\n")
    elif "AudioCommand::ApplyAllChannelTunings { tunings } => {" in line:
        out.append("                AudioCommand::ApplyAllChannelTunings { tunings } => {\n")
    elif "for (channel, delay_ms, eq_bands) in tunings {" in line:
        out.append("                    for (channel, delay_ms, eq_bands, phase_invert, gain_db) in tunings {\n")
    else:
        out.append(line)
        if "mixer.channel_dsp[channel].update_eq_targets(&eq_bands, mixer.sample_rate as f32);" in line:
            indent = line[:len(line) - len(line.lstrip())]
            out.append(f"{indent}mixer.channel_dsp[channel].is_phase_inverted = phase_invert;\n")
            out.append(f"{indent}mixer.channel_dsp[channel].target_gain_db = gain_db;\n")

with open("rust/src/audio/engine.rs", "w") as f:
    f.writelines(out)

# 3. Update simple.rs
with open("rust/src/api/simple.rs", "r") as f:
    content = f.read()

content = content.replace(
    "pub fn api_apply_channel_tuning(channel: u32, delay_ms: f32, eq_bands: Vec<EqBand>) -> Result<(), AtmosError> {",
    "pub fn api_apply_channel_tuning(channel: u32, delay_ms: f32, eq_bands: Vec<EqBand>, phase_invert: bool, gain_db: f32) -> Result<(), AtmosError> {"
)

content = content.replace(
    "AudioCommand::ApplyChannelTuning {\n            channel: channel as usize,\n            delay_ms,\n            eq_bands,\n        }",
    "AudioCommand::ApplyChannelTuning {\n            channel: channel as usize,\n            delay_ms,\n            eq_bands,\n            phase_invert,\n            gain_db,\n        }"
)

# ApplyAllChannelTunings
content = content.replace(
    "pub fn api_apply_all_channel_tunings(tunings: Vec<(u32, f32, Vec<EqBand>)>) -> Result<(), AtmosError> {",
    "pub fn api_apply_all_channel_tunings(tunings: Vec<(u32, f32, Vec<EqBand>, bool, f32)>) -> Result<(), AtmosError> {"
)

content = content.replace(
    "let mut mapped = Vec::with_capacity(tunings.len());\n    for (ch, d, eqs) in tunings {\n        mapped.push((ch as usize, d, eqs));\n    }",
    "let mut mapped = Vec::with_capacity(tunings.len());\n    for (ch, d, eqs, pi, g) in tunings {\n        mapped.push((ch as usize, d, eqs, pi, g));\n    }"
)

with open("rust/src/api/simple.rs", "w") as f:
    f.write(content)
