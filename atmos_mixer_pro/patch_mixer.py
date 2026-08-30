with open('rust/src/audio/mixer.rs', 'r') as f:
    content = f.read()

content = content.replace("channel_dsp[ch_idx].gain_db = setting.gain_db;", "channel_dsp[ch_idx].set_gain_db(setting.gain_db);")
content = content.replace("channel_dsp[ch_idx1].gain_db = setting.gain_db;", "channel_dsp[ch_idx1].set_gain_db(setting.gain_db);")
content = content.replace("channel_dsp[ch_idx2].gain_db = setting.gain_db;", "channel_dsp[ch_idx2].set_gain_db(setting.gain_db);")

with open('rust/src/audio/mixer.rs', 'w') as f:
    f.write(content)

print("mixer.rs patched")
