import re

with open("rust/src/audio/engine.rs", "r") as f:
    content = f.read()

# For SetChannelEq
content = content.replace("AudioCommand::SetChannelEq { channel, bands } => {",
"AudioCommand::SetChannelEq { channel, bands } => {\n                    let _ = mixer.spatial_gc_tx.try_send(crate::audio::mixer::SpatialGarbage::EqBands(bands.clone()));")

# Oops clone allocates! I must do it without clone if possible. Or I can just pass the whole `cmd` if I can extract what I need and repackage without allocations. 
