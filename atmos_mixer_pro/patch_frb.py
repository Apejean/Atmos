with open("rust/src/frb_generated.rs", "r") as f:
    content = f.read()

content = content.replace(
    "return crate::common::config::ChannelSetting {\n            enabled: api_enabled,\n            custom_name: api_custom_name,\n            delay_ms: api_delay_ms,\n            eq_bands: api_eq_bands,\n            position: api_position,\n        };",
    "return crate::common::config::ChannelSetting {\n            enabled: api_enabled,\n            custom_name: api_custom_name,\n            delay_ms: api_delay_ms,\n            eq_bands: api_eq_bands,\n            position: api_position,\n            phase_invert: false,\n            gain_db: 0.0,\n        };"
)

with open("rust/src/frb_generated.rs", "w") as f:
    f.write(content)
