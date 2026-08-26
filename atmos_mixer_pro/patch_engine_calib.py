with open("rust/src/audio/engine.rs", "r") as f:
    lines = f.readlines()

out_lines = []
for line in lines:
    out_lines.append(line)
    if "AudioCommand::ApplyChannelTuning { channel, delay_ms, eq_bands } => {" in line:
        pass # Actually we need to change ApplyChannelTuning definition
    if "AudioCommand::ApplyAllChannelTunings { tunings } => {" in line:
        pass # And this one
