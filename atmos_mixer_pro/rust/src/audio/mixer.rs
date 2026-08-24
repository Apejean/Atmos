use crate::audio::player::SoundInstance;
use crate::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use crate::audio::dsp::dsp_utils::ChannelDspState;

pub enum SpatialGarbage {
    RoomZones(Vec<crate::common::config::RoomZone>),
    Trajectory(Option<crate::common::config::Trajectory>),
    ChannelPositions(Vec<Option<crate::common::config::Point3D>>),
}

pub struct DuckingState {
    pub is_ducking: bool,
    pub ducking_weight: f32, // 1.0 down to 0.3
}

pub struct StartupMuteRamp {
    pub current_gain: f32, // 0.0 -> 1.0 (3초간 서서히 상승)
    pub ramp_step: f32,
}
impl StartupMuteRamp {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            current_gain: 0.0,
            ramp_step: 1.0 / (sample_rate * 3.0), // 3초 분량 샘플 스텝
        }
    }
    #[inline(always)]
    pub fn apply(&mut self, sample: f32) -> f32 {
        if self.current_gain < 1.0 {
            self.current_gain = (self.current_gain + self.ramp_step).min(1.0);
        }
        sample * self.current_gain // 부팅 초기 스피커 충격음 100% 차단
    }
}

pub struct AudioMixer {
    pub instances: Vec<Option<SoundInstance>>, // Fixed capacity object pool
    pub sample_rate: u32,
    pub ducking: DuckingState,
    pub gc_sender: crossbeam_channel::Sender<SoundInstance>,
    pub buf_gc_tx: crossbeam_channel::Sender<Vec<f32>>,
    pub spatial_gc_tx: crossbeam_channel::Sender<SpatialGarbage>,
    pub room_volumes: Vec<Option<(u32, f32)>>,
    pub local_recycle: Vec<Vec<f32>>,
    pub startup_ramp: StartupMuteRamp,
    pub master_mute: bool,
    pub channel_dsp: Vec<ChannelDspState>,
    pub channel_positions: Vec<Option<crate::common::config::Point3D>>,
    pub room_zones: Vec<crate::common::config::RoomZone>,
    pub trajectory: Option<crate::common::config::Trajectory>,
    pub master_headroom_db: f32,
    pub peak_limiter_enabled: bool,
    pub limiters: Vec<crate::audio::limiter::PeakLimiter>,
    pub temp_room_vols: Vec<f32>,
    pub temp_room_vols_target: Vec<f32>,
    pub channel_spatial_gains: Vec<f32>,
    pub channel_spatial_gains_target: Vec<f32>,
    pub temp_spatial_weights: Vec<f32>,
    pub analysis_tx: Option<rtrb::Producer<f32>>,
    pub temp_vals: Vec<f32>,
    pub master_clock: f64,
    pub spatializer: Option<crate::audio::spatial::Spatializer3D>,
    pub reverb: crate::audio::reverb::VirtualRoomReverb,
    pub binaural: crate::audio::binaural::VirtualMixRoomBinaural,
}

impl AudioMixer {
    pub fn new(sample_rate: u32, channels: usize, gc_sender: crossbeam_channel::Sender<SoundInstance>, analysis_tx: Option<rtrb::Producer<f32>>) -> Self {
        let (buf_gc_tx, buf_gc_rx) = crossbeam_channel::bounded::<Vec<f32>>(8192);
        std::thread::spawn(move || {
            while let Ok(_buf) = buf_gc_rx.recv() {
                // Buffer is dropped here in a background thread, preventing heap deallocation in the audio thread
            }
        });

        let (spatial_gc_tx, spatial_gc_rx) = crossbeam_channel::bounded::<SpatialGarbage>(1024);
        std::thread::spawn(move || {
            while let Ok(_garbage) = spatial_gc_rx.recv() {
                // Vecs drop here, preventing OS allocations in the audio thread
            }
        });

        let mut instances = Vec::with_capacity(4096);
        for _ in 0..4096 {
            instances.push(None);
        }
        let mut channel_dsp = Vec::with_capacity(channels);
        for _ in 0..channels {
            channel_dsp.push(ChannelDspState::new());
        }
        let mut channel_positions = vec![None; channels];
        let mut room_zones = Vec::new();
        let mut trajectory = None;
        let mut master_headroom_db = 0.0;
        let mut peak_limiter_enabled = true;
        let mut limiters = Vec::with_capacity(channels);
        for _ in 0..channels {
            limiters.push(crate::audio::limiter::PeakLimiter::new(sample_rate as f32, 1.0, 500.0, 0.99));
        }

        if let Ok(config_guard) = GLOBAL_STATE.config.read() {
            if let Some(config) = config_guard.as_ref() {
                for (&ch_key, setting) in &config.mono_configs {
                    if ch_key > 0 {
                        let ch_idx = (ch_key - 1) as usize;
                        if ch_idx < channel_dsp.len() {
                            channel_dsp[ch_idx].update_delay_target(setting.delay_ms);
                            channel_dsp[ch_idx].update_eq_targets(&setting.eq_bands.clone(), sample_rate as f32);
                            channel_positions[ch_idx] = setting.position.clone();
                        }
                    }
                }
                for (&ch_key, setting) in &config.stereo_configs {
                    if ch_key > 0 {
                        let ch_idx1 = (ch_key - 1) as usize;
                        let ch_idx2 = ch_idx1 + 1;
                        if ch_idx1 < channel_dsp.len() {
                            channel_dsp[ch_idx1].update_delay_target(setting.delay_ms);
                            channel_dsp[ch_idx1].update_eq_targets(&setting.eq_bands.clone(), sample_rate as f32);
                            channel_positions[ch_idx1] = setting.position.clone();
                        }
                        if ch_idx2 < channel_dsp.len() {
                            channel_dsp[ch_idx2].update_delay_target(setting.delay_ms);
                            channel_dsp[ch_idx2].update_eq_targets(&setting.eq_bands.clone(), sample_rate as f32);
                            channel_positions[ch_idx2] = setting.position.clone();
                        }
                    }
                }
                for (&ch_key, setting) in &config.multi_configs {
                    if ch_key > 0 {
                        let ch_idx_base = (ch_key - 1) as usize;
                        for i in 0..6 {
                            let ch_idx = ch_idx_base + i;
                            if ch_idx < channel_dsp.len() {
                                channel_dsp[ch_idx].update_delay_target(setting.delay_ms);
                                channel_dsp[ch_idx].update_eq_targets(&setting.eq_bands.clone(), sample_rate as f32);
                                channel_positions[ch_idx] = setting.position.clone();
                            }
                        }
                    }
                }
                
                // Auto Boundary EQ and Acoustic Delay is now handled by recalculate_spatial_dsp()
                // --------------------------------------------------------------------
                room_zones = config.room_zones.clone();
                trajectory = config.global_trajectory.clone();
                master_headroom_db = config.master_headroom_db;
                peak_limiter_enabled = config.peak_limiter_enabled;
            }
        }

        let mut mixer = Self {
            instances,
            sample_rate,
            ducking: DuckingState {
                is_ducking: false,
                ducking_weight: 1.0,
            },
            gc_sender,
            buf_gc_tx,
            spatial_gc_tx,
            room_volumes: vec![None; channels],
            local_recycle: Vec::with_capacity(8192),
            startup_ramp: StartupMuteRamp::new(sample_rate as f32),
            master_mute: false,
            channel_dsp,
            channel_positions,
            room_zones,
            trajectory,
            master_headroom_db,
            peak_limiter_enabled,
            limiters,
            temp_room_vols: vec![1.0; 4096],
            temp_room_vols_target: vec![1.0; 4096],
            channel_spatial_gains: vec![1.0; channels],
            channel_spatial_gains_target: vec![1.0; channels],
            temp_spatial_weights: vec![0.0; channels],
            analysis_tx,
            temp_vals: vec![0.0; channels],
            master_clock: 0.0,
            spatializer: None,
            reverb: crate::audio::reverb::VirtualRoomReverb::new(sample_rate as f32),
            binaural: crate::audio::binaural::VirtualMixRoomBinaural::new(channels, 1024), // Using 1024 as default block size for now
        };
        
        mixer.recalculate_spatial_dsp();
        
        mixer
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
        
        self.master_clock += frames as f64;

        if let Ok(config_guard) = GLOBAL_STATE.config.try_read() {
            if let Some(config) = config_guard.as_ref() {
                self.master_headroom_db = config.master_headroom_db;
                self.peak_limiter_enabled = config.peak_limiter_enabled;
            }
        }

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

        self.temp_room_vols_target.fill(1.0);
        for (i, inst_opt) in self.instances.iter_mut().enumerate() {
            if let Some(inst) = inst_opt {
                if inst.is_playing {
                    for (rid, rvol) in self.room_volumes.iter().flatten() {
                        if *rid == inst.room_id {
                            self.temp_room_vols_target[i] = *rvol;
                            break;
                        }
                    }
                    
                    if inst.output_channel == usize::MAX && inst.current_position.is_some() {
                        let active_ch = out_channels.min(self.channel_positions.len());
                        if inst.spatial_gains.len() != active_ch {
                            inst.spatial_gains.resize(active_ch, 0.0);
                            inst.spatial_gains_target.resize(active_ch, 0.0);
                        }
                        
                        let pos = inst.current_position.as_ref().unwrap();
                        let mut sum_sq = 0.0;
                        let mut min_dist = f32::MAX;
                        let blur_radius = 2.0f32;
                        
                        for ch in 0..active_ch {
                            if let Some(c_pos) = &self.channel_positions[ch] {
                                let dx = c_pos.x - pos.x;
                                let dy = c_pos.y - pos.y;
                                let dz = c_pos.z - pos.z;
                                let dist = (dx*dx + dy*dy + dz*dz).sqrt();
                                if dist < min_dist { min_dist = dist; }
                                let weight = 1.0 / (dist.powi(2) + blur_radius.powi(2));
                                inst.spatial_gains_target[ch] = weight;
                                sum_sq += weight * weight;
                            } else {
                                inst.spatial_gains_target[ch] = 0.0;
                            }
                        }
                        
                        let norm_factor = if sum_sq > 0.0 { 1.0 / sum_sq.sqrt() } else { 0.0 };
                        let distance_attenuation = 1.0 / min_dist.max(1.0);
                        for ch in 0..active_ch {
                            let pan_ratio = inst.spatial_gains_target[ch] * norm_factor;
                            inst.spatial_gains_target[ch] = pan_ratio * distance_attenuation;
                        }
                    }
                }
            }
        }

        let active_ch = out_channels.min(self.channel_spatial_gains_target.len());
        
        self.channel_spatial_gains_target[..active_ch].fill(1.0);
        self.temp_spatial_weights[..active_ch].fill(0.0);

        let mut sum_sq = 0.0;
        let mut min_dist = f32::MAX;
        let blur_radius = 2.0f32;

        for ch in 0..active_ch {
            let mut gain = 1.0;
            if ch < self.channel_positions.len() {
                if let Some(pos) = &self.channel_positions[ch] {
                    // 1. Point-in-Room Auto-Binding
                    let mut bound_room_id = None;
                    for zone in &self.room_zones {
                        if pos.x >= zone.boundary_min.x && pos.x <= zone.boundary_max.x &&
                           pos.y >= zone.boundary_min.y && pos.y <= zone.boundary_max.y {
                            bound_room_id = Some(zone.room_id);
                            break;
                        }
                    }
                    if let Some(rid) = bound_room_id {
                        for (room_id, rvol) in self.room_volumes.iter().flatten() {
                            if *room_id == rid {
                                gain *= *rvol;
                                break;
                            }
                        }
                    }
                    
                    // 2. Trajectory DBAP Weight Collection
                    if let Some(traj) = &self.trajectory {
                        let mut in_target_room = true;
                        if let Some(target_zone_str) = &traj.target_room_zone_id {
                            let target_rid = target_zone_str.parse::<u32>().unwrap_or_else(|_| crate::common::utils::hash_id(target_zone_str));
                            if bound_room_id != Some(target_rid) {
                                in_target_room = false;
                            }
                        }

                        if in_target_room {
                            let dx = pos.x - traj.current_position.x;
                            let dy = pos.y - traj.current_position.y;
                            let dz = pos.z - traj.current_position.z;
                            let dist = (dx*dx + dy*dy + dz*dz).sqrt();
                            
                            if dist < min_dist {
                                min_dist = dist;
                            }
                            
                            let weight = 1.0 / (dist.powi(2) + blur_radius.powi(2));
                            self.temp_spatial_weights[ch] = weight;
                            sum_sq += weight * weight;
                        } else {
                            self.temp_spatial_weights[ch] = 0.0;
                        }
                    }
                }
            }
            self.channel_spatial_gains_target[ch] = gain;
        }

        // Apply DBAP Normalization & Global Distance Attenuation
        if self.trajectory.is_some() {
            let norm_factor = if sum_sq > 0.0 { 1.0 / sum_sq.sqrt() } else { 0.0 };
            let distance_attenuation = 1.0 / min_dist.max(1.0);

            for ch in 0..active_ch {
                let pan_ratio = self.temp_spatial_weights[ch] * norm_factor;
                self.channel_spatial_gains_target[ch] *= pan_ratio * distance_attenuation;
            }
        }

        let mut temp_vals = std::mem::take(&mut self.temp_vals);

        for frame in 0..frames {
            // Anti-zipper smoothing for spatial automation (~4ms time constant)
            for ch in 0..active_ch {
                self.channel_spatial_gains[ch] += 0.005 * (self.channel_spatial_gains_target[ch] - self.channel_spatial_gains[ch]);
                if ch < GLOBAL_STATE.spatial_gains.len() {
                    GLOBAL_STATE.spatial_gains[ch].store(self.channel_spatial_gains[ch].to_bits(), Ordering::Relaxed);
                }
            }

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

                self.temp_room_vols[i] += 0.005 * (self.temp_room_vols_target[i] - self.temp_room_vols[i]);
                
                if instance.output_channel == usize::MAX {
                    let active_ch = out_channels.min(self.channel_positions.len());
                    for ch in 0..active_ch {
                        instance.spatial_gains[ch] += 0.005 * (instance.spatial_gains_target[ch] - instance.spatial_gains[ch]);
                    }
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

                let smoothed_volume = instance.volume_smoother.get_next();
                let mut current_vol = smoothed_volume * instance.fade_weight * self.temp_room_vols[i];
                if instance.is_loop {
                    current_vol *= self.ducking.ducking_weight; // Ducking only affects BGM
                }

                let step = instance.stream_sample_rate as f64 / self.sample_rate as f64;

                let channels = (instance.stream_channels as usize).max(1);

                let mut idx_f = instance.cursor;
                let mut idx_base = idx_f as usize;
                let mut frac = (idx_f - (idx_base as f64)) as f32;
                let mut idx_i = idx_base * channels;

                let ch_limit = channels.min(temp_vals.len());
                let vals = &mut temp_vals[..ch_limit];
                let mut has_sample = false;

                if let Some(stream_rx) = &mut instance.stream_receiver {
                    if idx_i >= instance.stream_buffer.len() {
                        match stream_rx.pop() {
                            Ok(new_chunk) => {
                                instance.anti_click_multiplier = 1.0;
                                let frames_in_chunk = (instance.stream_buffer.len() / channels) as f64;
                                let old_chunk = std::mem::replace(&mut instance.stream_buffer, new_chunk);
                                if let Err(e) = self.buf_gc_tx.try_send(old_chunk) {
                                    let v = e.into_inner();
                                    if self.local_recycle.len() < self.local_recycle.capacity() {
                                        self.local_recycle.push(v);
                                    } else {
                                        let _ = v;
                                    }
                                }
                                instance.cursor -= frames_in_chunk;
                                if instance.cursor < 0.0 {
                                    instance.cursor = 0.0;
                                }
                                idx_f = instance.cursor;
                                idx_base = idx_f as usize;
                                frac = (idx_f - (idx_base as f64)) as f32;
                                idx_i = idx_base * channels;
                            }
                            Err(rtrb::PopError::Empty) => {}
                        }
                    }

                    if idx_i < instance.stream_buffer.len() {
                        has_sample = true;
                        let next_idx = if idx_i + channels < instance.stream_buffer.len() {
                            idx_i + channels
                        } else {
                            idx_i
                        };
                        for (ch, val) in vals.iter_mut().enumerate().take(ch_limit) {
                            let idx0 = if idx_i >= channels { idx_i - channels } else { idx_i };
                            let idx3 = if next_idx + channels < instance.stream_buffer.len() { next_idx + channels } else { next_idx };

                            let s0 = instance.stream_buffer.get(idx0 + ch).copied().unwrap_or(0.0);
                            let s1 = instance.stream_buffer.get(idx_i + ch).copied().unwrap_or(0.0);
                            let s2 = instance.stream_buffer.get(next_idx + ch).copied().unwrap_or(s1);
                            let s3 = instance.stream_buffer.get(idx3 + ch).copied().unwrap_or(s2);
                            
                            *val = crate::audio::dsp::dsp_utils::interpolate_hermite(s0, s1, s2, s3, frac);
                            if let Some(last) = instance.last_samples.get_mut(ch) {
                                *last = *val;
                            }
                        }
                    } else {
                        has_sample = true;
                        instance.anti_click_multiplier *= 0.95; // 1-pole non-linear fade
                        for (ch, val) in vals.iter_mut().enumerate().take(ch_limit) {
                            *val = instance.last_samples.get(ch).copied().unwrap_or(0.0) * instance.anti_click_multiplier;
                        }
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
                        for (ch, val) in vals.iter_mut().enumerate().take(ch_limit) {
                            let idx0 = if idx_i >= channels { idx_i - channels } else { if instance.is_loop && data.samples.len() >= channels { data.samples.len() - channels } else { idx_i } };
                            let idx3 = if next_idx + channels < data.samples.len() { next_idx + channels } else { if instance.is_loop { 0 } else { next_idx } };

                            let s0 = data.samples.get(idx0 + ch).copied().unwrap_or(0.0);
                            let s1 = data.samples.get(idx_i + ch).copied().unwrap_or(0.0);
                            let s2 = data.samples.get(next_idx + ch).copied().unwrap_or(s1);
                            let s3 = data.samples.get(idx3 + ch).copied().unwrap_or(s2);
                            *val = crate::audio::dsp::dsp_utils::interpolate_hermite(s0, s1, s2, s3, frac);
                        }
                    }
                }

                if has_sample {
                    if instance.output_channel == usize::MAX && instance.current_position.is_some() {
                        // Object Mode
                        let mut sum = 0.0;
                        for val in vals.iter().take(ch_limit) {
                            sum += *val;
                        }
                        let mono_val = sum / ch_limit as f32;
                        
                        let active_ch = out_channels.min(self.channel_positions.len());
                        for hw_ch in 0..active_ch {
                            let is_enabled = if hw_ch < GLOBAL_STATE.enabled_channels.len() {
                                GLOBAL_STATE.enabled_channels[hw_ch].load(Ordering::Relaxed)
                            } else {
                                false
                            };
                            let out_idx = frame * out_channels + hw_ch;
                            if is_enabled && out_idx < output.len() {
                                output[out_idx] += mono_val * current_vol * instance.spatial_gains[hw_ch];
                            }
                        }
                    } else if !instance.output_stereo && ch_limit > 1 {
                        // Downmix all to Mono for backwards compatibility if output_stereo is false
                        let mut sum = 0.0;
                        for val in vals.iter().take(ch_limit) {
                            sum += *val;
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
                        for (ch, val) in vals.iter().enumerate().take(ch_limit) {
                            let hw_ch = instance.output_channel + ch;
                            if hw_ch < out_channels {
                                let is_enabled = if hw_ch < GLOBAL_STATE.enabled_channels.len() {
                                    GLOBAL_STATE.enabled_channels[hw_ch].load(Ordering::Relaxed)
                                } else {
                                    false
                                };
                                let out_idx = frame * out_channels + hw_ch;
                                if is_enabled && out_idx < output.len() {
                                    output[out_idx] += val * current_vol;
                                }
                            }
                        }

                        // Mono file -> Stereo Out upmix for backwards compatibility
                        if ch_limit == 1 && instance.output_stereo {
                            let hw_ch_r = instance.output_channel + 1;
                            if hw_ch_r < out_channels {
                                let out_idx_r = frame * out_channels + hw_ch_r;
                                if out_idx_r < output.len() {
                                    output[out_idx_r] += vals[0] * current_vol * self.channel_spatial_gains[hw_ch_r];
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
        
        self.temp_vals = temp_vals;

        // Apply Channel DSP
        let fs = self.sample_rate as f32;
        let dsp_limit = self.channel_dsp.len().min(out_channels);
        for ch in 0..dsp_limit {
            let is_enabled = if ch < GLOBAL_STATE.enabled_channels.len() {
                GLOBAL_STATE.enabled_channels[ch].load(Ordering::Relaxed)
            } else {
                false
            };
            if !is_enabled { continue; }

            for frame in 0..frames {
                let sample_idx = frame * out_channels + ch;
                if sample_idx < output.len() {
                    let mut val = output[sample_idx];
                    val = self.channel_dsp[ch].process(val, fs);
                    output[sample_idx] = val;
                }
            }
        }
        
        // Apply Binaural Processing (De-interleaves, convolves, and re-interleaves to Ch0 & Ch1)
        self.binaural.process_interleaved(output, out_channels);

        // Apply Global Reverb to Ch0 and Ch1
        if out_channels >= 2 && self.reverb.mix > 0.0 {
            for frame in 0..frames {
                let idx_l = frame * out_channels + 0;
                let idx_r = frame * out_channels + 1;
                if idx_r < output.len() {
                    let (rl, rr) = self.reverb.process_stereo(output[idx_l], output[idx_r]);
                    output[idx_l] = rl;
                    output[idx_r] = rr;
                }
            }
        }

        // Compute VU levels (Peak per channel) and apply soft clipping
        let headroom_gain = 10.0f32.powf(self.master_headroom_db / 20.0);
        
        for ch in 0..out_channels {
            let mut peak: f32 = 0.0;
            for frame in 0..frames {
                let sample_idx = frame * out_channels + ch;
                if sample_idx < output.len() {
                    let mut val = output[sample_idx];

                    // Apply Master Headroom Padding
                    val *= headroom_gain;

                    // Output Peak Limiter Guard
                    if self.peak_limiter_enabled && ch < self.limiters.len() {
                        val = self.limiters[ch].process(val);
                    } else {
                        // Fallback absolute hard clamp if limiter is disabled
                        if val > 1.0 {
                            val = 0.99;
                        } else if val < -1.0 {
                            val = -0.99;
                        }
                    }
                    val = self.startup_ramp.apply(val);
                    if self.master_mute {
                        val = 0.0;
                    }
                    if GLOBAL_STATE.is_failover_mode.load(Ordering::Relaxed) {
                        val *= 0.01; // -40dB safety pad
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
        
        // Send to Analysis Thread (Lock-free)
        if let Some(tx) = &mut self.analysis_tx {
            let available = tx.slots();
            if available >= output.len() {
                if let Ok(mut chunk) = tx.write_chunk(output.len()) {
                    let (slice1, slice2) = chunk.as_mut_slices();
                    let len1 = slice1.len();
                    slice1.copy_from_slice(&output[..len1]);
                    if !slice2.is_empty() {
                        slice2.copy_from_slice(&output[len1..]);
                    }
                    chunk.commit_all();
                }
            }
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

    pub fn recalculate_spatial_dsp(&mut self) {
        if let Ok(config_guard) = crate::core::state::GLOBAL_STATE.config.try_read() {
            if let Some(config) = config_guard.as_ref() {
                // 1. Collect base delay and EQs from config
                let mut base_delays = vec![0.0_f32; self.channel_dsp.len()];
                let mut base_eqs = vec![Vec::new(); self.channel_dsp.len()];
                
                let mut apply_config = |ch_key: u32, setting: &crate::common::config::ChannelSetting, ch_offset: usize, count: usize| {
                    if ch_key > 0 {
                        let ch_idx_base = (ch_key - 1) as usize + ch_offset;
                        for i in 0..count {
                            let ch_idx = ch_idx_base + i;
                            if ch_idx < self.channel_dsp.len() {
                                base_delays[ch_idx] = setting.delay_ms;
                                base_eqs[ch_idx] = setting.eq_bands.clone();
                            }
                        }
                    }
                };

                for (&ch_key, setting) in &config.mono_configs { apply_config(ch_key, setting, 0, 1); }
                for (&ch_key, setting) in &config.stereo_configs { apply_config(ch_key, setting, 0, 2); }
                for (&ch_key, setting) in &config.multi_configs { apply_config(ch_key, setting, 0, 6); }

                // 2. Find target pos
                let target_pos = if let Some(traj) = &self.trajectory {
                    Some(traj.current_position.clone())
                } else {
                    None
                };

                // 3. Pre-calculate max_dist for time alignment
                let mut max_dist = 0.0_f32;
                let mut channel_dists = vec![0.0_f32; self.channel_positions.len()];
                
                let target_room_id = if let Some(traj) = &self.trajectory {
                    traj.target_room_zone_id.as_ref().map(|s| s.parse::<u32>().unwrap_or_else(|_| crate::common::utils::hash_id(s)))
                } else {
                    None
                };

                for ch_idx in 0..self.channel_positions.len() {
                    if let Some(pos) = &self.channel_positions[ch_idx] {
                        let mut bound_room_id = None;
                        for zone in &self.room_zones {
                            if pos.x >= zone.boundary_min.x && pos.x <= zone.boundary_max.x &&
                               pos.y >= zone.boundary_min.y && pos.y <= zone.boundary_max.y {
                                bound_room_id = Some(zone.room_id);
                                break;
                            }
                        }

                        let in_target_room = match target_room_id {
                            Some(target_id) => bound_room_id == Some(target_id),
                            None => true,
                        };

                        if in_target_room {
                            let t_pos = if let Some(tp) = &target_pos {
                                tp.clone()
                            } else {
                                // Find zone center
                                let mut cx = pos.x;
                                let mut cz = pos.z;
                                if let Some(rid) = bound_room_id {
                                    for zone in &self.room_zones {
                                        if zone.room_id == rid {
                                            cx = (zone.boundary_min.x + zone.boundary_max.x) / 2.0;
                                            cz = (zone.boundary_min.y + zone.boundary_max.y) / 2.0;
                                            break;
                                        }
                                    }
                                }
                                crate::common::config::Point3D { x: cx, y: pos.y, z: cz, ..Default::default() }
                            };
                            
                            let dist = crate::audio::acoustic::distance_3d(pos, &t_pos);
                            channel_dists[ch_idx] = dist;
                            if dist > max_dist {
                                max_dist = dist;
                            }
                        } else {
                            channel_dists[ch_idx] = -1.0;
                        }
                    }
                }

                // 4. Apply Time Alignment & Dynamic Off-axis EQ
                for ch_idx in 0..self.channel_dsp.len() {
                    let dist = channel_dists[ch_idx];
                    if dist < 0.0 {
                        continue; // Not participating in spatial DSP (isolated)
                    }

                    if let Some(pos) = &self.channel_positions[ch_idx] {
                        let mut matched_zone = None;
                        for zone in &self.room_zones {
                            if pos.x >= zone.boundary_min.x && pos.x <= zone.boundary_max.x &&
                               pos.y >= zone.boundary_min.y && pos.y <= zone.boundary_max.y {
                                matched_zone = Some(zone);
                                break;
                            }
                        }

                        let t_pos = if let Some(tp) = &target_pos {
                            tp.clone()
                        } else if let Some(zone) = matched_zone {
                            let cx = (zone.boundary_min.x + zone.boundary_max.x) / 2.0;
                            let cz = (zone.boundary_min.y + zone.boundary_max.y) / 2.0; 
                            crate::common::config::Point3D { x: cx, y: pos.y, z: cz, ..Default::default() }
                        } else {
                            pos.clone()
                        };
                        
                        // Phase 2: Time Alignment Formula
                        let delay_seconds = (max_dist - dist) / crate::audio::acoustic::SPEED_OF_SOUND_M_S;
                        let acoustic_delay_ms = delay_seconds * 1000.0;
                        let zone_delay = matched_zone.map(|z| z.boundary_delay_ms).unwrap_or(0.0);
                        
                        let base_delay = base_delays[ch_idx];
                        self.channel_dsp[ch_idx].update_delay_target(base_delay + acoustic_delay_ms + zone_delay);

                        // Phase 2: Dispersion Angle off-axis EQ roll-off
                        let mut dynamic_eq = None;
                        if pos.dispersion_angle > 0.0 {
                            let forward_vec = crate::audio::acoustic::calculate_forward_vector(pos.pitch_tilt, pos.yaw_rotation);
                            let to_target_vec = crate::audio::acoustic::vector_from_to(pos, &t_pos);
                            
                            let f_norm = crate::audio::acoustic::normalize(&forward_vec);
                            let t_norm = crate::audio::acoustic::normalize(&to_target_vec);
                            let theta = crate::audio::acoustic::angle_between_vectors(&f_norm, &t_norm);
                            
                            let half_dispersion = pos.dispersion_angle / 2.0;
                            if theta > half_dispersion {
                                let excess_angle = theta - half_dispersion;
                                let roll_off_gain = - (excess_angle * 0.5).min(24.0); // max -24dB attenuation
                                
                                dynamic_eq = Some(crate::common::config::EqBand {
                                    enabled: true,
                                    freq: 4000.0,
                                    gain: roll_off_gain,
                                    q_factor: 0.707,
                                    filter_type: crate::common::config::EqType::HighShelf,
                                });
                            }
                        }

                        // Apply Base EQ, Boundary EQ, and Dynamic EQ
                        let mut final_eqs = base_eqs[ch_idx].clone();
                        
                        if let Some(zone) = matched_zone {
                            for b_eq in &zone.boundary_eq_bands {
                                let mut already_exists = false;
                                for band in &mut final_eqs {
                                    if band.enabled && (band.freq - b_eq.freq).abs() < 1.0 && band.filter_type == b_eq.filter_type {
                                        already_exists = true;
                                        band.gain = b_eq.gain;
                                        break;
                                    }
                                }
                                if !already_exists {
                                    final_eqs.push(b_eq.clone());
                                }
                            }
                        }

                        if let Some(dyn_eq) = dynamic_eq {
                            let mut already_exists = false;
                            for band in &mut final_eqs {
                                if band.enabled && (band.freq - dyn_eq.freq).abs() < 1.0 && band.filter_type == dyn_eq.filter_type {
                                    already_exists = true;
                                    band.gain = dyn_eq.gain;
                                    break;
                                }
                            }
                            if !already_exists {
                                final_eqs.push(dyn_eq);
                            }
                        }

                        // Make sure we don't exceed MAX_EQ_BANDS by truncating
                        if final_eqs.len() > crate::audio::dsp::dsp_utils::MAX_EQ_BANDS {
                            final_eqs.truncate(crate::audio::dsp::dsp_utils::MAX_EQ_BANDS);
                        }

                        let sr = self.sample_rate as f32;
                        self.channel_dsp[ch_idx].update_eq_targets(&final_eqs, sr);
                    }
                }
            }
        }
    }
}
