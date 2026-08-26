import re

with open("rust/src/audio/engine.rs", "r") as f:
    content = f.read()

# I need to insert UpdateOutputConfig handling in the engine match loop.
# Find `AudioCommand::UpdateTrajectoryPosition { position } => {` and insert it.

new_block = """                AudioCommand::UpdateTrajectoryPosition { position } => {
                    mixer.trajectory_pos = position;
                }
                AudioCommand::UpdateOutputConfig { channel, mute, solo, invert_phase, gain_db, delay_ms } => {
                    if channel < mixer.channel_dsp.len() {
                        mixer.channel_dsp[channel].is_muted = mute;
                        mixer.channel_dsp[channel].is_soloed = solo;
                        mixer.channel_dsp[channel].is_phase_inverted = invert_phase;
                        mixer.channel_dsp[channel].target_gain_db = gain_db;
                        mixer.channel_dsp[channel].update_delay_target(delay_ms);
                        
                        let mut any_soloed = false;
                        for ch in mixer.channel_dsp.iter() {
                            if ch.is_soloed {
                                any_soloed = true;
                                break;
                            }
                        }
                        mixer.any_soloed = any_soloed;
                    }
                }"""

content = re.sub(r"AudioCommand::UpdateTrajectoryPosition \{ position \} => \{\s*mixer\.trajectory_pos = position;\s*\}", new_block, content)

with open("rust/src/audio/engine.rs", "w") as f:
    f.write(content)
