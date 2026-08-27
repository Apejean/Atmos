with open("rust/src/api/acoustics.rs", "r") as f:
    content = f.read()

# Fix UpdateSpatialConfig sending in api_set_transmission_loss
old_block = """    if let Some(config) = config_cloned {
        let _ = GLOBAL_STATE.command_sender.send(AudioCommand::UpdateSpatialConfig {
            channel_positions: config.channel_positions.clone(),
            room_zones: config.room_zones.clone(),
            trajectory: config.global_trajectory.clone(),
            track_positions: config.track_positions.clone(),
        });
    }"""
new_block = """    // Frontend should call api_update_spatial_config_json to update the DSP engine."""

content = content.replace(old_block, new_block)

# Fix unused variable warnings
content = content.replace("let mut config_cloned = None;", "")
content = content.replace("config_cloned = Some(config.clone());", "")

with open("rust/src/api/acoustics.rs", "w") as f:
    f.write(content)

