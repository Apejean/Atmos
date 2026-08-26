import re

with open('rust/src/common/config_routing.rs', 'r') as f:
    content = f.read()

if "pub eq_bands:" not in content:
    content = content.replace("pub gain_db: f32,", 
"""pub gain_db: f32,
    #[serde(default)]
    pub eq_bands: Vec<crate::common::config::EqBand>,""")

with open('rust/src/common/config_routing.rs', 'w') as f:
    f.write(content)
