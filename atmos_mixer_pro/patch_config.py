with open("rust/src/common/config.rs", "r") as f:
    lines = f.readlines()

out_lines = []
for i, line in enumerate(lines):
    if line.strip() == "pub position: Option<Point3D>,":
        out_lines.append(line)
        out_lines.append("    #[serde(default)]\n")
        out_lines.append("    pub phase_invert: bool,\n")
        out_lines.append("    #[serde(default)]\n")
        out_lines.append("    pub gain_db: f32,\n")
    else:
        out_lines.append(line)

with open("rust/src/common/config.rs", "w") as f:
    f.writelines(out_lines)
