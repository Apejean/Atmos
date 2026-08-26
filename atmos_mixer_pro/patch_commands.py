with open("rust/src/common/commands.rs", "r") as f:
    lines = f.readlines()

out_lines = []
for i, line in enumerate(lines):
    if line.strip() == "pub enum AudioCommand {":
        out_lines.append(line)
        out_lines.append("    UpdateOutputRouting {\n")
        out_lines.append("        payload: crate::common::config_routing::OutputRoutingPayload,\n")
        out_lines.append("    },\n")
    else:
        out_lines.append(line)

with open("rust/src/common/commands.rs", "w") as f:
    f.writelines(out_lines)
