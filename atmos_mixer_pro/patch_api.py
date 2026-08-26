with open("rust/src/api/simple.rs", "r") as f:
    lines = f.readlines()

out_lines = []
for i, line in enumerate(lines):
    if line.strip() == "use crate::common::config::Point3D;":
        out_lines.append(line)
        out_lines.append("use crate::common::config_routing::OutputRoutingPayload;\n")
        out_lines.append("pub fn api_update_output_routing(json_payload: String) -> Result<(), AtmosError> {\n")
        out_lines.append("    let payload: OutputRoutingPayload = serde_json::from_str(&json_payload).map_err(|e| AtmosError {\n")
        out_lines.append("        message: format!(\"Failed to parse output routing JSON: {}\", e),\n")
        out_lines.append("    })?;\n")
        out_lines.append("    crate::core::state::GLOBAL_STATE.command_sender.send(crate::common::commands::AudioCommand::UpdateOutputRouting { payload }).map_err(|e| AtmosError {\n")
        out_lines.append("        message: format!(\"Failed to send UpdateOutputRouting: {}\", e),\n")
        out_lines.append("    })?;\n")
        out_lines.append("    Ok(())\n")
        out_lines.append("}\n")
    else:
        out_lines.append(line)

with open("rust/src/api/simple.rs", "w") as f:
    f.writelines(out_lines)
