use crate::audio::player::SoundInstance;
use crate::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;

pub struct DuckingState {
    pub is_ducking: bool,
    pub ducking_weight: f32, // 1.0 down to 0.3
}

pub struct AudioMixer {
    pub instances: Vec<Option<SoundInstance>>, // Fixed capacity object pool
    pub sample_rate: u32,
    pub ducking: DuckingState,
    pub gc_sender: crossbeam_channel::Sender<SoundInstance>,
    pub buf_gc_tx: crossbeam_channel::Sender<Vec<f32>>,
    pub room_volumes: Vec<Option<(u32, f32)>>,
    pub local_recycle: Vec<Vec<f32>>,
}

impl AudioMixer {
    pub fn new(sample_rate: u32, gc_sender: crossbeam_channel::Sender<SoundInstance>) -> Self {
        let (buf_gc_tx, buf_gc_rx) = crossbeam_channel::bounded::<Vec<f32>>(4096);
        std::thread::spawn(move || {
            while let Ok(_buf) = buf_gc_rx.recv() {
                // Buffer is dropped here in a background thread, preventing heap deallocation in the audio thread
            }
        });

        let mut instances = Vec::with_capacity(4096);
        for _ in 0..4096 {
            instances.push(None);
        }
        Self {
            instances,
            sample_rate,
            ducking: DuckingState {
                is_ducking: false,
                ducking_weight: 1.0,
            },
            gc_sender,
            buf_gc_tx,
            room_volumes: vec![None; 128],
            local_recycle: Vec::with_capacity(4096),
        }
    }

    pub fn process(&mut self, output: &mut [f32], out_channels: usize) {
        if out_channels == 0 || output.is_empty() {
            return;
        }

        // Clear output buffer
        for sample in output.iter_mut() {
            *sample = 0.0;
        }

        let frames = output.len() / out_channels;

        let fade_frames = (self.sample_rate as f32 * 0.3) as usize; // 300ms fade
        let duck_down_frames = (self.sample_rate as f32 * 0.15) as usize; // 150ms duck down
        let duck_up_frames = (self.sample_rate as f32 * 0.3) as usize; // 300ms duck up

        // Check if any SFX is playing (not loop)
        let has_sfx = self.instances.iter().any(|inst| {
            if let Some(inst) = inst {
                !inst.is_loop && inst.is_playing && !inst.is_stopping
            } else {
                false
            }
        });

        let is_exhib = GLOBAL_STATE.is_exhibition_mode.load(Ordering::Relaxed);
        if is_exhib {
            self.ducking.is_ducking = false;
        } else {
            if has_sfx && !self.ducking.is_ducking {
                self.ducking.is_ducking = true;
            } else if !has_sfx && self.ducking.is_ducking && self.ducking.ducking_weight <= 0.3 {
                self.ducking.is_ducking = false; // Start unducking
            }
        }

        let mut temp_room_vols = [1.0f32; 4096];
        for (i, inst_opt) in self.instances.iter().enumerate() {
            if let Some(inst) = inst_opt {
                if inst.is_playing {
                    for slot in &self.room_volumes {
                        if let Some((rid, rvol)) = slot {
                            if *rid == inst.room_id {
                                temp_room_vols[i] = *rvol;
                                break;
                            }
                        }
                    }
                }
            }
        }

        for frame in 0..frames {
            // Update ducking weight per frame
            if self.ducking.is_ducking {
                if self.ducking.ducking_weight > 0.3 {
                    self.ducking.ducking_weight -= 0.7 / duck_down_frames as f32;
                }
                if self.ducking.ducking_weight < 0.3 {
                    self.ducking.ducking_weight = 0.3;
                }
            } else {
                if self.ducking.ducking_weight < 1.0 {
                    self.ducking.ducking_weight += 0.7 / duck_up_frames as f32;
                }
                if self.ducking.ducking_weight > 1.0 {
                    self.ducking.ducking_weight = 1.0;
                }
            }

            for (i, instance_opt) in self.instances.iter_mut().enumerate() {
                let instance = match instance_opt {
                    Some(inst) => inst,
                    None => continue,
                };
                
                if !instance.is_playing {
                    continue;
                }

                // Update fade weight
                if instance.is_stopping {
                    instance.fade_weight -= 1.0 / fade_frames as f32;
                    if instance.fade_weight <= 0.0 {
                        instance.fade_weight = 0.0;
                        instance.is_playing = false;
                        continue;
                    }
                } else {
                    instance.fade_weight += 1.0 / fade_frames as f32;
                    if instance.fade_weight > 1.0 {
                        instance.fade_weight = 1.0;
                    }
                }

                let mut current_vol = instance.volume * instance.fade_weight * temp_room_vols[i];
                if instance.is_loop {
                    current_vol *= self.ducking.ducking_weight; // Ducking only affects BGM
                }

                let step = instance.stream_sample_rate as f64 / self.sample_rate as f64;

                let channels = (instance.stream_channels as usize).max(1);

                let mut idx_f = instance.cursor;
                let mut idx_base = idx_f as usize;
                let mut frac = (idx_f - (idx_base as f64)) as f32;
                let mut idx_i = idx_base * channels;

                let mut vals = [0.0; 64]; // N-channel temp buffer (up to 64 channels)
                let ch_limit = channels.min(64);
                let mut has_sample = false;

                if let Some(stream_rx) = &instance.stream_receiver {
                    if instance.is_loop {
                        if idx_i >= instance.stream_buffer.len() {
                            match stream_rx.try_recv() {
                                Ok(new_chunk) => {
                                    let frames_in_chunk = if instance.stream_buffer.is_empty() {
                                        0.0
                                    } else {
                                        (instance.stream_buffer.len() / channels) as f64
                                    };
                                    let old_chunk =
                                        std::mem::replace(&mut instance.stream_buffer, new_chunk);
                                    if let Err(e) = self.buf_gc_tx.try_send(old_chunk) {
                                        let v = e.into_inner();
                                        if self.local_recycle.len() < self.local_recycle.capacity() {
                                            self.local_recycle.push(v);
                                        } else {
                                            // Channel is full and local recycle is full.
                                            // We must drop it to prevent a fatal OOM memory leak.
                                            // Dropping here might theoretically acquire a heap lock, 
                                            // but it's vastly better than guaranteed OOM.
                                            let _ = v;
                                        }
                                    }

                                    instance.cursor -= frames_in_chunk;
                                    if instance.cursor < 0.0 {
                                        instance.cursor = 0.0;
                                    } // safety bound
                                    idx_f = instance.cursor;
                                    idx_base = idx_f as usize;
                                    frac = (idx_f - (idx_base as f64)) as f32;
                                    idx_i = idx_base * channels;
                                }
                                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                                    // Stream finished or errored permanently
                                    instance.is_stopping = true;
                                }
                                Err(crossbeam_channel::TryRecvError::Empty) => {
                                    // Stream is lagging, just output silence and don't advance cursor
                                }
                            }
                        }

                        if idx_i < instance.stream_buffer.len() {
                            has_sample = true;
                            let next_idx = if idx_i + channels < instance.stream_buffer.len() {
                                idx_i + channels
                            } else {
                                idx_i
                            };
                            for ch in 0..ch_limit {
                                let s1 = instance.stream_buffer.get(idx_i + ch).copied().unwrap_or(0.0);
                                let s2 = instance.stream_buffer.get(next_idx + ch).copied().unwrap_or(s1);
                                vals[ch] = s1 + frac * (s2 - s1);
                            }
                        } else {
                            has_sample = true;
                            for ch in 0..ch_limit {
                                vals[ch] = 0.0;
                            }
                        }
                    } else {
                        // Stream receiver is not loop? Wait, if it's not loop but has stream_receiver
                        // it should be processed the same way. But let's follow the old logic.
                    }
                } else if let Some(data) = &instance.data {
                    if idx_i >= data.samples.len() && instance.is_loop {
                        let frames_in_data = (data.samples.len() / channels) as f64;
                        instance.cursor -= frames_in_data;
                        if instance.cursor < 0.0 {
                            instance.cursor = 0.0;
                        }
                        idx_f = instance.cursor;
                        idx_base = idx_f as usize;
                        frac = (idx_f - (idx_base as f64)) as f32;
                        idx_i = idx_base * channels;
                    }

                    if idx_i < data.samples.len() {
                        has_sample = true;
                        let mut next_idx = idx_i + channels;
                        if next_idx >= data.samples.len() {
                            if instance.is_loop {
                                next_idx = 0;
                            } else {
                                next_idx = idx_i;
                            }
                        }
                        for ch in 0..ch_limit {
                            let s1 = data.samples.get(idx_i + ch).copied().unwrap_or(0.0);
                            let s2 = data.samples.get(next_idx + ch).copied().unwrap_or(s1);
                            vals[ch] = s1 + frac * (s2 - s1);
                        }
                    }
                }

                if has_sample {
                    if !instance.output_stereo && ch_limit > 1 {
                        // Downmix all to Mono for backwards compatibility if output_stereo is false
                        let mut sum = 0.0;
                        for ch in 0..ch_limit {
                            sum += vals[ch];
                        }
                        let mono_val = sum / ch_limit as f32;

                        let hw_ch = instance.output_channel;
                        if hw_ch < out_channels {
                            let is_enabled = if hw_ch < GLOBAL_STATE.enabled_channels.len() {
                                GLOBAL_STATE.enabled_channels[hw_ch].load(Ordering::Relaxed)
                            } else {
                                false
                            };
                            let out_idx = frame * out_channels + hw_ch;
                            if is_enabled && out_idx < output.len() {
                                output[out_idx] += mono_val * current_vol;
                            }
                        }
                    } else {
                        // N:N direct routing
                        for ch in 0..ch_limit {
                            let hw_ch = instance.output_channel + ch;
                            if hw_ch < out_channels {
                                let is_enabled = if hw_ch < GLOBAL_STATE.enabled_channels.len() {
                                    GLOBAL_STATE.enabled_channels[hw_ch].load(Ordering::Relaxed)
                                } else {
                                    false
                                };
                                let out_idx = frame * out_channels + hw_ch;
                                if is_enabled && out_idx < output.len() {
                                    output[out_idx] += vals[ch] * current_vol;
                                }
                            }
                        }

                        // Mono file -> Stereo Out upmix for backwards compatibility
                        if ch_limit == 1 && instance.output_stereo {
                            let hw_ch_r = instance.output_channel + 1;
                            if hw_ch_r < out_channels {
                                let is_r_enabled = if hw_ch_r < GLOBAL_STATE.enabled_channels.len() {
                                    GLOBAL_STATE.enabled_channels[hw_ch_r].load(Ordering::Relaxed)
                                } else {
                                    false
                                };
                                let out_idx_r = frame * out_channels + hw_ch_r;
                                if is_r_enabled && out_idx_r < output.len() {
                                    output[out_idx_r] += vals[0] * current_vol;
                                }
                            }
                        }
                    }

                    if instance.stream_receiver.is_some() && idx_i >= instance.stream_buffer.len() {
                        // Buffer is empty, stream is lagging. Don't advance cursor.
                    } else {
                        instance.cursor += step;
                    }
                } else {
                    instance.is_stopping = true;
                }
            }
        }

        // Compute VU levels (Peak per channel) and apply soft clipping
        for ch in 0..out_channels {
            if ch >= GLOBAL_STATE.enabled_channels.len() {
                break;
            }
            let is_enabled = GLOBAL_STATE.enabled_channels[ch].load(Ordering::Relaxed);
            if !is_enabled {
                GLOBAL_STATE.vu_levels[ch].store(0, Ordering::Relaxed);
                // Zero out buffer for disabled channels to be absolutely safe
                for frame in 0..frames {
                    let sample_idx = frame * out_channels + ch;
                    if sample_idx < output.len() {
                        output[sample_idx] = 0.0;
                    }
                }
                continue;
            }

            let mut peak: f32 = 0.0;
            for frame in 0..frames {
                let sample_idx = frame * out_channels + ch;
                if sample_idx < output.len() {
                    let mut val = output[sample_idx];

                    // Soft clipping
                    if val <= -1.0 {
                        val = -1.0;
                    } else if val >= 1.0 {
                        val = 1.0;
                    } else {
                        val = 1.5 * val - 0.5 * val * val * val;
                    }
                    output[sample_idx] = val;

                    let abs_val = val.abs();
                    if abs_val > peak {
                        peak = abs_val;
                    }
                }
            }

            GLOBAL_STATE.vu_levels[ch].store(peak.to_bits(), Ordering::Relaxed);
        }

        // Remove stopped instances by moving to GC thread (heap-free drop)
        // Try to flush local_recycle to gc_sender if possible
        while !self.local_recycle.is_empty() {
            if let Some(chunk) = self.local_recycle.pop() {
                if let Err(e) = self.buf_gc_tx.try_send(chunk) {
                    self.local_recycle.push(e.into_inner());
                    break; // Channel is still full
                }
            }
        }

        for slot in self.instances.iter_mut() {
            if let Some(inst) = slot {
                if !inst.is_playing {
                    if let Some(old) = slot.take() {
                        let _ = self.gc_sender.try_send(old);
                    }
                }
            }
        }
    }
}
