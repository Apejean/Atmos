with open("rust/src/audio/mixer.rs", "r") as f:
    content = f.read()

# Make sure temp vectors are there
if "pub temp_base_delays: Vec<f32>," not in content:
    content = content.replace("pub channel_positions: Vec<Option<crate::common::config::Point3D>>,", "pub channel_positions: Vec<Option<crate::common::config::Point3D>>,\n    pub temp_base_delays: Vec<f32>,\n    pub temp_base_eqs: Vec<Vec<crate::common::config::EqBand>>,\n    pub temp_channel_dists: Vec<f32>,")
    
if "temp_base_delays: vec![0.0; channels]," not in content:
    content = content.replace("channel_positions: vec![None; channels],", "channel_positions: vec![None; channels],\n            temp_base_delays: vec![0.0; channels],\n            temp_base_eqs: vec![Vec::new(); channels],\n            temp_channel_dists: vec![0.0; channels],")

with open("rust/src/audio/mixer.rs", "w") as f:
    f.write(content)
