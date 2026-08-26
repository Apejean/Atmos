with open("rust/src/audio/engine.rs", "r") as f:
    lines = f.readlines()

out_lines = []
for i, line in enumerate(lines):
    if line.strip() == "AudioCommand::UpdateSpatialConfig { channel_positions, room_zones, trajectory, track_positions } => {":
        out_lines.append("                AudioCommand::UpdateOutputRouting { payload } => {\n")
        out_lines.append("                    let mut new_solos = Vec::new();\n")
        out_lines.append("                    for (i, config) in payload.channels.iter().enumerate() {\n")
        out_lines.append("                        if i < mixer.channel_dsp.len() {\n")
        out_lines.append("                            mixer.channel_dsp[i].is_muted = config.is_muted;\n")
        out_lines.append("                            mixer.channel_dsp[i].is_soloed = config.is_soloed;\n")
        out_lines.append("                            mixer.channel_dsp[i].is_phase_inverted = config.is_phase_inverted;\n")
        out_lines.append("                            mixer.channel_dsp[i].update_delay_target(config.delay_ms);\n")
        out_lines.append("                            mixer.channel_dsp[i].target_gain_db = config.gain_db;\n")
        out_lines.append("                            if config.is_soloed {\n")
        out_lines.append("                                new_solos.push(i);\n")
        out_lines.append("                            }\n")
        out_lines.append("                        }\n")
        out_lines.append("                    }\n")
        out_lines.append("                    mixer.soloed_channels = new_solos;\n")
        out_lines.append("                }\n")
    out_lines.append(line)

with open("rust/src/audio/engine.rs", "w") as f:
    f.writelines(out_lines)
