import re

with open("rust/src/audio/engine.rs", "r") as f:
    content = f.read()

# Update ApplyAllChannelTunings
content = content.replace("                AudioCommand::ApplyAllChannelTunings { tunings } => {", 
"""                AudioCommand::ApplyAllChannelTunings { tunings } => {
                    let _ = mixer.spatial_gc_tx.try_send(crate::audio::mixer::SpatialGarbage::Command(AudioCommand::ApplyAllChannelTunings { tunings: tunings.clone() }));""")

# Wait, `tunings.clone()` inside the audio thread allocates!
# I can't just recreate it. I need to move it into GC.
