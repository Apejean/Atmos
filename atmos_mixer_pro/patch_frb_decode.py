with open("rust/src/frb_generated.rs", "r") as f:
    content = f.read()

content = content.replace(
    "return crate::common::config::ChannelSetting {\n            enabled: var_enabled,\n            custom_name: var_customName,\n            delay_ms: var_delayMs,\n            eq_bands: var_eqBands,\n            position: var_position,\n        };",
    "return crate::common::config::ChannelSetting {\n            enabled: var_enabled,\n            custom_name: var_customName,\n            delay_ms: var_delayMs,\n            eq_bands: var_eqBands,\n            position: var_position,\n            phase_invert: false,\n            gain_db: 0.0,\n        };"
)

with open("rust/src/frb_generated.rs", "w") as f:
    f.write(content)
