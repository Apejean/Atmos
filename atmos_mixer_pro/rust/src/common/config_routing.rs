use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChannelRoutingConfig {
    pub id: u32,
    pub name: String,
    pub is_muted: bool,
    pub is_soloed: bool,
    pub is_phase_inverted: bool,
    pub delay_ms: f32,
    pub gain_db: f32,
    #[serde(default)]
    pub eq_bands: Vec<crate::common::config::EqBand>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct OutputRoutingPayload {
    pub channels: Vec<ChannelRoutingConfig>,
}
