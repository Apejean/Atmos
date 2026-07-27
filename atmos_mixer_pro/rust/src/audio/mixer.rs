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
    pub channel_spatial_gains: Vec<f32>,
    pub channel_spatial_gains_target: Vec<f32>,
    pub temp_spatial_weights: Vec<f32>,
    pub rta_analyzer: crate::audio::rta::RtaAnalyzer,
    pub mono_mix_buffer: Vec<f32>,
    pub temp_vals: Vec<f32>,
    pub master_clock: f64,
}

impl AudioMixer {
    pub fn new(sample_rate: u32, channels: usize, gc_sender: crossbeam_channel::Sender<SoundInstance>) -> Self {
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
            limiters.push(crate::audio::limiter::PeakLimiter::new(sample_rate as f32, 0.1, 100.0, 1.0));
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
                
                // --- V1.0.80 Calibration DSP: Auto Boundary EQ and Acoustic Delay ---
                // We use distance from center of active RoomZone or polygon centroid as delay reference point
                for ch_idx in 0..channel_dsp.len() {
                    if let Some(pos) = &channel_positions[ch_idx] {
                        let mut matched_zone = None;
                        for zone in &config.room_zones {
                            if pos.x >= zone.boundary_min.x && pos.x <= zone.boundary_max.x &&
                               pos.y >= zone.boundary_min.y && pos.y <= zone.boundary_max.y {
                                matched_zone = Some(zone);
                                break;
                            }
                        }

                        if let Some(zone) = matched_zone {
                            // Centroid for delay ref (center of AABB)
                            let cx = (zone.boundary_min.x + zone.boundary_max.x) / 2.0;
                            let cz = (zone.boundary_min.y + zone.boundary_max.y) / 2.0; // Using Y as Z for 2D top-down
                            let center = crate::common::config::Point3D { x: cx, y: pos.y, z: cz };
                            let dist = crate::audio::acoustic::distance_3d(pos, &center);
                            let acoustic_delay = crate::audio::acoustic::calculate_acoustic_delay_ms(dist);

                            // Acoustic delay logic
                            // base_delay should be 0.0 or from the config, not accumulated every loop iteration.
                            let base_delay = 0.0;
                            channel_dsp[ch_idx].update_delay_target(base_delay + acoustic_delay + zone.boundary_delay_ms);

                            // Boundary EQ
                            if !zone.boundary_eq_bands.is_empty() {
                                let mut new_targets = channel_dsp[ch_idx].target_bands.clone();
                                for b_eq in &zone.boundary_eq_bands {
                                    // Check if this specific EQ band already exists
                                    let mut already_exists = false;
                                    for band in &new_targets {
                                        if band.enabled && (band.freq - b_eq.freq).abs() < 1.0 && band.filter_type == b_eq.filter_type {
                                            already_exists = true;
                                            break;
                                        }
                                    }
                                    
                                    if !already_exists {
                                        if new_targets.is_empty() {
                                            new_targets.push(b_eq.clone());
                                        } else {
                                            let mut applied = false;
                                            for band in &mut new_targets {
                                                if !band.enabled {
                                                    *band = b_eq.clone();
                                                    applied = true;
                                                    break;
                                                }
                                            }
                                            if !applied && new_targets.len() < crate::audio::dsp::dsp_utils::MAX_EQ_BANDS {
                                                new_targets.push(b_eq.clone());
                                            }
                                        }
                                    }
                                }
                                channel_dsp[ch_idx].update_eq_targets(&new_targets, sample_rate as f32);
                            }
                        }
                    }
                }
                // --------------------------------------------------------------------
                room_zones = config.room_zones.clone();
                trajectory = config.global_trajectory.clone();
                master_headroom_db = config.master_headroom_db;
                peak_limiter_enabled = config.peak_limiter_enabled;
            }
        }

        let mixer = Self {
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
            channel_spatial_gains: vec![1.0; channels],
            channel_spatial_gains_target: vec![1.0; channels],
            temp_spatial_weights: vec![0.0; channels],
            rta_analyzer: crate::audio::rta::RtaAnalyzer::new(),
            mono_mix_buffer: vec![0.0; 8192], // Pre-allocated to prevent heap allocation in callback
            temp_vals: vec![0.0; channels],
            master_clock: 0.0,
        };
        
        {
            let mut lock = GLOBAL_STATE.rta_magnitudes_ref.write().unwrap();
            *lock = Some(mixer.rta_analyzer.get_magnitudes_arc());
        }
        
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

        self.temp_room_vols.fill(1.0);
        for (i, inst_opt) in self.instances.iter().enumerate() {
            if let Some(inst) = inst_opt {
                if inst.is_playing {
                    for (rid, rvol) in self.room_volumes.iter().flatten() {
                        if *rid == inst.room_id {
                            self.temp_room_vols[i] = *rvol;
                            break;
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

        // Phase 2: Centralized Global Audio Clock / Block Fetch Loop
        // We fetch chunks once per process block for all instances to ensure 100% sync
        // and avoid lock-free channel overhead in the tight inner loop.
        for instance_opt in self.instances.iter_mut() {
            if let Some(instance) = instance_opt {
                if !instance.is_playing { continue; }
                if let Some(stream_rx) = &instance.stream_receiver {
                    let channels = (instance.stream_channels as usize).max(1);
                    let idx_i = instance.cursor as usize * channels;
                    if idx_i >= instance.stream_buffer.len() {
                        match stream_rx.try_recv() {
                            Ok(new_chunk) => {
                                let frames_in_chunk = if instance.stream_buffer.is_empty() {
                                    0.0
                                } else {
                                    (instance.stream_buffer.len() / channels) as f64
                                };
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
                            }
                            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                                instance.is_stopping = true;
                            }
                            Err(crossbeam_channel::TryRecvError::Empty) => {}
                        }
                    }
                }
            }
        }

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

                if instance.stream_receiver.is_some() {
                    // Try_recv was done in the outer Block Fetch loop.

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
                        }
                    } else {
                        has_sample = true;
                        for val in vals.iter_mut().take(ch_limit) {
                            *val = 0.0;
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
                    if !instance.output_stereo && ch_limit > 1 {
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
                                output[out_idx] += mono_val * current_vol * self.channel_spatial_gains[hw_ch];
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
                                    output[out_idx] += val * current_vol * self.channel_spatial_gains[hw_ch];
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

        // Compute VU levels (Peak per channel) and apply soft clipping
        let headroom_gain = 10.0f32.powf(self.master_headroom_db / 20.0);
        
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
                    output[sample_idx] = val;

                    let abs_val = val.abs();
                    if abs_val > peak {
                        peak = abs_val;
                    }
                }
            }

            GLOBAL_STATE.vu_levels[ch].store(peak.to_bits(), Ordering::Relaxed);
        }
        
        // Tap Master Output (Channel 0 and 1 downmix) to RTA Analyzer
        if out_channels > 0 {
            // Ensure capacity to prevent dynamic allocation
            if self.mono_mix_buffer.len() < frames {
                self.mono_mix_buffer.resize(frames, 0.0);
            }
            let mono_mix = &mut self.mono_mix_buffer[..frames];
            
            for (frame, item) in mono_mix.iter_mut().enumerate() {
                let mut sum = 0.0;
                let mut count = 0;
                // Mix up to first 2 channels for RTA visualization
                for ch in 0..out_channels.min(2) {
                    if GLOBAL_STATE.enabled_channels[ch].load(Ordering::Relaxed) {
                        let sample_idx = frame * out_channels + ch;
                        if sample_idx < output.len() {
                            sum += output[sample_idx];
                            count += 1;
                        }
                    }
                }
                if count > 0 {
                    *item = sum / count as f32;
                } else {
                    *item = 0.0;
                }
            }
            self.rta_analyzer.process_samples(mono_mix);
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
