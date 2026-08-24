import re

with open("rust/src/audio/engine.rs", "r") as f:
    content = f.read()

# For SetChannelEq
old_set_eq = """                AudioCommand::SetChannelEq { channel, bands } => {
                    if channel < mixer.channel_dsp.len() {
                        mixer.channel_dsp[channel].update_eq_targets(&bands, mixer.sample_rate as f32);
                    }
                }"""
new_set_eq = """                AudioCommand::SetChannelEq { channel, bands } => {
                    if channel < mixer.channel_dsp.len() {
                        mixer.channel_dsp[channel].update_eq_targets(&bands, mixer.sample_rate as f32);
                    }
                    let _ = mixer.spatial_gc_tx.try_send(crate::audio::mixer::SpatialGarbage::EqBands(bands));
                }"""
content = content.replace(old_set_eq, new_set_eq)

# For ApplyChannelTuning
old_apply_eq = """                AudioCommand::ApplyChannelTuning { channel, delay_ms, eq_bands } => {
                    if channel < mixer.channel_dsp.len() {
                        mixer.channel_dsp[channel].update_delay_target(delay_ms);
                        mixer.channel_dsp[channel].update_eq_targets(&eq_bands, mixer.sample_rate as f32);
                    }
                }"""
new_apply_eq = """                AudioCommand::ApplyChannelTuning { channel, delay_ms, eq_bands } => {
                    if channel < mixer.channel_dsp.len() {
                        mixer.channel_dsp[channel].update_delay_target(delay_ms);
                        mixer.channel_dsp[channel].update_eq_targets(&eq_bands, mixer.sample_rate as f32);
                    }
                    let _ = mixer.spatial_gc_tx.try_send(crate::audio::mixer::SpatialGarbage::EqBands(eq_bands));
                }"""
content = content.replace(old_apply_eq, new_apply_eq)

# For ApplyAllChannelTunings
old_apply_all = """                AudioCommand::ApplyAllChannelTunings { tunings } => {
                    for (ch, delay_ms, eq_bands) in tunings {
                        if ch < mixer.channel_dsp.len() {
                            mixer.channel_dsp[ch].update_delay_target(delay_ms);
                            mixer.channel_dsp[ch].update_eq_targets(&eq_bands, mixer.sample_rate as f32);
                        }
                    }
                }"""
new_apply_all = """                AudioCommand::ApplyAllChannelTunings { tunings } => {
                    for (ch, delay_ms, eq_bands) in &tunings {
                        if *ch < mixer.channel_dsp.len() {
                            mixer.channel_dsp[*ch].update_delay_target(*delay_ms);
                            mixer.channel_dsp[*ch].update_eq_targets(eq_bands, mixer.sample_rate as f32);
                        }
                    }
                    let _ = mixer.spatial_gc_tx.try_send(crate::audio::mixer::SpatialGarbage::AllTunings(tunings));
                }"""
content = content.replace(old_apply_all, new_apply_all)

# For UpdateSpatialConfig
old_track_pos = """                    for inst in mixer.instances.iter_mut().flatten() {
                        if let Some(pos) = track_positions.get(&inst.track_id_str) {
                            inst.current_position = Some(pos.clone());
                        }
                    }
                    mixer.recalculate_spatial_dsp();
                }"""
new_track_pos = """                    for inst in mixer.instances.iter_mut().flatten() {
                        if let Some(pos) = track_positions.get(&inst.track_id_str) {
                            inst.current_position = Some(pos.clone());
                        }
                    }
                    mixer.recalculate_spatial_dsp();
                    let _ = mixer.spatial_gc_tx.try_send(crate::audio::mixer::SpatialGarbage::TrackPositions(track_positions));
                }"""
content = content.replace(old_track_pos, new_track_pos)

with open("rust/src/audio/engine.rs", "w") as f:
    f.write(content)
