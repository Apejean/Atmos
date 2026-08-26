with open("rust/src/audio/mixer.rs", "r") as f:
    lines = f.readlines()

out_lines = []
for i, line in enumerate(lines):
    if line.strip() == "pub channel_spatial_gains_phase: Vec<f32>,":
        out_lines.append(line)
        out_lines.append("    pub soloed_channels: Vec<usize>,\n")
    elif line.strip() == "channel_spatial_gains_phase: vec![1.0; channels],":
        out_lines.append(line)
        out_lines.append("            soloed_channels: Vec::new(),\n")
    elif line.strip() == "let mut val = output[out_idx];":
        out_lines.append(line)
        out_lines.append("                let mut ch_is_muted = self.channel_dsp[ch].is_muted;\n")
        out_lines.append("                if !self.soloed_channels.is_empty() && !self.soloed_channels.contains(&ch) {\n")
        out_lines.append("                    ch_is_muted = true;\n")
        out_lines.append("                }\n")
        out_lines.append("                if ch_is_muted {\n")
        out_lines.append("                    output[out_idx] = 0.0;\n")
        out_lines.append("                    continue;\n")
        out_lines.append("                }\n")
    else:
        out_lines.append(line)

with open("rust/src/audio/mixer.rs", "w") as f:
    f.writelines(out_lines)
