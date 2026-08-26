with open("rust/src/audio/dsp.rs", "r") as f:
    lines = f.readlines()

out_lines = []
for i, line in enumerate(lines):
    if line.strip() == "pub delay_write_idx: usize,":
        out_lines.append(line)
        out_lines.append("        pub is_muted: bool,\n")
        out_lines.append("        pub is_soloed: bool,\n")
        out_lines.append("        pub is_phase_inverted: bool,\n")
        out_lines.append("        pub target_gain_db: f32,\n")
        out_lines.append("        pub current_gain_db: f32,\n")
    elif line.strip() == "delay_buffer: vec![0.0; DELAY_BUFFER_SIZE],":
        out_lines.append(line)
        out_lines.append("                is_muted: false,\n")
        out_lines.append("                is_soloed: false,\n")
        out_lines.append("                is_phase_inverted: false,\n")
        out_lines.append("                target_gain_db: 0.0,\n")
        out_lines.append("                current_gain_db: 0.0,\n")
    elif line.strip() == "sample = self.air_absorption.process(sample);":
        out_lines.append(line)
        out_lines.append("            // Gain processing\n")
        out_lines.append("            self.current_gain_db += 0.001 * (self.target_gain_db - self.current_gain_db);\n")
        out_lines.append("            let gain_mult = 10.0f32.powf(self.current_gain_db / 20.0);\n")
        out_lines.append("            sample *= gain_mult;\n")
        out_lines.append("            if self.is_phase_inverted {\n")
        out_lines.append("                sample = -sample;\n")
        out_lines.append("            }\n")
    else:
        out_lines.append(line)

with open("rust/src/audio/dsp.rs", "w") as f:
    f.writelines(out_lines)
