import re

with open("rust/src/audio/mixer.rs", "r") as f:
    content = f.read()

# Replace soloed_channels with any_soloed if it's there
content = content.replace("pub soloed_channels: Vec<usize>,", "pub any_soloed: bool,")
content = content.replace("soloed_channels: Vec::new(),", "any_soloed: false,")

# Add the mute/solo check before channel_dsp process
old_block = """                if sample_idx < output.len() {
                    let mut val = output[sample_idx];
                    val = self.channel_dsp[ch].process(val, fs);
                    output[sample_idx] = val;
                }"""

new_block = """                if sample_idx < output.len() {
                    let mut ch_is_muted = self.channel_dsp[ch].is_muted;
                    if self.any_soloed && !self.channel_dsp[ch].is_soloed {
                        ch_is_muted = true;
                    }
                    if ch_is_muted {
                        output[sample_idx] = 0.0;
                        continue;
                    }
                    let mut val = output[sample_idx];
                    val = self.channel_dsp[ch].process(val, fs);
                    output[sample_idx] = val;
                }"""

content = content.replace(old_block, new_block)

with open("rust/src/audio/mixer.rs", "w") as f:
    f.write(content)
