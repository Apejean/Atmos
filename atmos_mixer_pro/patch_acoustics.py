import re

with open("rust/src/api/acoustics.rs", "r") as f:
    content = f.read()

# Make sure to import AudioCommand
if "crate::common::commands::AudioCommand;" not in content:
    content = content.replace("use crate::core::state::GLOBAL_STATE;", "use crate::core::state::GLOBAL_STATE;\nuse crate::common::commands::AudioCommand;")

# Replace api_set_global_reverb
old_reverb = """    // 추후 오디오 엔진으로 업데이트 명령 전송 필요 시 AudioCommand 발송
    // if let Some(config) = config_cloned {
    //     let _ = GLOBAL_STATE.command_sender.send(AudioCommand::UpdateSpatialConfig { ... });
    // }"""
new_reverb = """    let _ = GLOBAL_STATE.command_sender.send(AudioCommand::SetReverbParams { mix, decay });"""
content = content.replace(old_reverb, new_reverb)

# Replace api_set_transmission_loss
old_loss = """    // if let Some(config) = config_cloned {
    //     let _ = GLOBAL_STATE.command_sender.send(AudioCommand::UpdateSpatialConfig { ... });
    // }"""
new_loss = """    if let Some(config) = config_cloned {
        let _ = GLOBAL_STATE.command_sender.send(AudioCommand::UpdateSpatialConfig {
            channel_positions: config.channel_positions.clone(),
            room_zones: config.room_zones.clone(),
            trajectory: config.global_trajectory.clone(),
            track_positions: config.track_positions.clone(),
        });
    }"""
content = content.replace(old_loss, new_loss)

# Add #[flutter_rust_bridge::frb(sync)] to api_set_global_reverb and api_set_transmission_loss
content = content.replace("pub fn api_set_global_reverb", "#[flutter_rust_bridge::frb(sync)]\npub fn api_set_global_reverb")
content = content.replace("pub fn api_set_transmission_loss", "#[flutter_rust_bridge::frb(sync)]\npub fn api_set_transmission_loss")

with open("rust/src/api/acoustics.rs", "w") as f:
    f.write(content)
