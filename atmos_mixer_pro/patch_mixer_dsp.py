import re

with open("rust/src/audio/mixer.rs", "r") as f:
    content = f.read()

# Add UpdateOutputConfig handler
handler_code = """
            AudioCommand::UpdateOutputConfig { channel, mute, solo, invert_phase, gain_db, delay_ms } => {
                if channel < self.channel_dsp.len() {
                    let dsp = &mut self.channel_dsp[channel];
                    dsp.is_muted = mute;
                    dsp.is_soloed = solo;
                    dsp.is_phase_inverted = invert_phase;
                    dsp.target_gain_db = gain_db;
                    dsp.target_delay_ms = delay_ms;
                }
            }"""

# Insert into the command matching logic inside mixer.rs
content = content.replace("AudioCommand::ApplyGlobalTuning { master_headroom_db, peak_limiter_enabled } => {", handler_code + "\n            AudioCommand::ApplyGlobalTuning { master_headroom_db, peak_limiter_enabled } => {")

with open("rust/src/audio/mixer.rs", "w") as f:
    f.write(content)
