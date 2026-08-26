import re

with open("rust/src/audio/mixer.rs", "r") as f:
    lines = f.readlines()

out_lines = []
for line in lines:
    out_lines.append(line)
    if "channel_dsp[ch_idx].update_delay_target(setting.delay_ms);" in line:
        indent = line[:len(line) - len(line.lstrip())]
        out_lines.append(f"{indent}channel_dsp[ch_idx].is_phase_inverted = setting.phase_invert;\n")
        out_lines.append(f"{indent}channel_dsp[ch_idx].target_gain_db = setting.gain_db;\n")
    elif "channel_dsp[ch_idx1].update_delay_target(setting.delay_ms);" in line:
        indent = line[:len(line) - len(line.lstrip())]
        out_lines.append(f"{indent}channel_dsp[ch_idx1].is_phase_inverted = setting.phase_invert;\n")
        out_lines.append(f"{indent}channel_dsp[ch_idx1].target_gain_db = setting.gain_db;\n")
    elif "channel_dsp[ch_idx2].update_delay_target(setting.delay_ms);" in line:
        indent = line[:len(line) - len(line.lstrip())]
        out_lines.append(f"{indent}channel_dsp[ch_idx2].is_phase_inverted = setting.phase_invert;\n")
        out_lines.append(f"{indent}channel_dsp[ch_idx2].target_gain_db = setting.gain_db;\n")
    elif "base_delays[ch_idx] = setting.delay_ms;" in line:
        indent = line[:len(line) - len(line.lstrip())]
        out_lines.append(f"{indent}self.channel_dsp[ch_idx].is_phase_inverted = setting.phase_invert;\n")
        out_lines.append(f"{indent}self.channel_dsp[ch_idx].target_gain_db = setting.gain_db;\n")

with open("rust/src/audio/mixer.rs", "w") as f:
    f.writelines(out_lines)
