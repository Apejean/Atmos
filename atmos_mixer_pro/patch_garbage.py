import re

with open("rust/src/audio/mixer.rs", "r") as f:
    content = f.read()

content = content.replace("pub enum SpatialGarbage {", "pub enum SpatialGarbage {\n    Command(crate::common::commands::AudioCommand),\n    TrackPositions(std::collections::HashMap<String, crate::common::config::Point3D>),\n    EqBands(Vec<crate::common::config::EqBand>),")

with open("rust/src/audio/mixer.rs", "w") as f:
    f.write(content)
