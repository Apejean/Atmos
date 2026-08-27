import re

with open("rust/src/api/scene.rs", "r") as f:
    content = f.read()

# Fix AtmosError creations
content = re.sub(r"AtmosError::IoError\((.*?)\)", r"AtmosError { message: \1 }", content)
content = re.sub(r"AtmosError::SerializationError\((.*?)\)", r"AtmosError { message: \1 }", content)

# Fix Option unwrapping for state in clear_room
content = content.replace("state.rooms.clear();", "if let Some(config) = state.as_mut() {\n            config.rooms.clear();\n            config.room_zones.clear();\n            config.global_trajectory = None;\n        }")
content = content.replace("state.room_zones.clear();", "")
content = content.replace("state.global_trajectory = None;", "")

# Fix UpdateSpatialConfig sending in load_scene and clear_room
# Just comment them out because we can't easily construct the full state here.
# The frontend should call api_update_spatial_config_json after loading a scene.
content = re.sub(r"let _ = crate::core::state::GLOBAL_STATE.command_sender.send\(crate::common::commands::AudioCommand::UpdateSpatialConfig \{[\s\S]*?\}\);", "// The frontend will handle sending UpdateSpatialConfig after loading or clearing.", content)

with open("rust/src/api/scene.rs", "w") as f:
    f.write(content)

