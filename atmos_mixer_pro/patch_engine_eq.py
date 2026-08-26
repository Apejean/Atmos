import re

with open("rust/src/audio/engine.rs", "r") as f:
    content = f.read()

old_block = """                            mixer.channel_dsp[i].is_phase_inverted = config.is_phase_inverted;
                            mixer.channel_dsp[i].update_delay_target(config.delay_ms);
                            mixer.channel_dsp[i].target_gain_db = config.gain_db;
                            if config.is_soloed {"""

new_block = """                            mixer.channel_dsp[i].is_phase_inverted = config.is_phase_inverted;
                            mixer.channel_dsp[i].update_delay_target(config.delay_ms);
                            mixer.channel_dsp[i].target_gain_db = config.gain_db;
                            mixer.channel_dsp[i].update_eq_targets(&config.eq_bands, mixer.sample_rate as f32);
                            if config.is_soloed {"""

content = content.replace(old_block, new_block)

with open("rust/src/audio/engine.rs", "w") as f:
    f.write(content)
