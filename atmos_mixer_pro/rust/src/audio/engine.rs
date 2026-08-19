use crate::audio::mixer::AudioMixer;
use crate::common::commands::AudioCommand;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{OutputCallbackInfo, SampleFormat, Stream, StreamConfig};

use std::sync::atomic::{AtomicBool, Ordering};

lazy_static::lazy_static! {
    pub static ref ENGINE_INIT_SIGNAL: AtomicBool = AtomicBool::new(false);
}

pub struct AudioEngine {
    stream: Option<Stream>,
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        if let Some(stream) = self.stream.take() {
            println!("🔥 [디버깅] 오디오 스트림 명시적 Pause 및 Drop 수행...");
            let _ = stream.pause();
            drop(stream);
            println!("✅ [디버깅] 오디오 스트림 Drop 완료!");
        }
    }
}

impl AudioEngine {
    pub fn new() -> Self {
        Self { stream: None }
    }
}

#[cfg(target_os = "windows")]
pub fn get_hosts(target_prefix: Option<&str>) -> Result<Vec<cpal::Host>, String> {
    let mut hosts = Vec::new();

    let req_asio = target_prefix.map_or(true, |p| p == "[ASIO]");
    let req_wasapi = target_prefix.map_or(true, |p| p == "[WASAPI]");

    if req_asio {
        match cpal::host_from_id(cpal::HostId::Asio) {
            Ok(host) => hosts.push(host),
            Err(e) => eprintln!("ASIO Load Error: {:?}", e),
        }
    }

    if req_wasapi {
        hosts.push(cpal::default_host()); // WASAPI is default on Windows
    }

    if hosts.is_empty() {
        hosts.push(cpal::default_host());
    }

    Ok(hosts)
}

#[cfg(not(target_os = "windows"))]
pub fn get_hosts(_target_prefix: Option<&str>) -> Result<Vec<cpal::Host>, String> {
    Ok(vec![cpal::default_host()])
}

#[cfg(target_os = "windows")]
pub fn apply_windows_admin_optimizations() {
    use windows::Win32::UI::Shell::IsUserAnAdmin;
    
    unsafe {
        if IsUserAnAdmin().as_bool() {
            println!("🔥 [디버깅] 관리자 권한 확인됨. MMCSS 스레드 승격 & RAM 고정 시도.");
            // 1. MMCSS (Pro Audio) 승격
            let mut task_index = 0;
            let class_name: Vec<u16> = "Pro Audio\0".encode_utf16().collect();
            let _handle = windows::Win32::System::Threading::AvSetMmThreadCharacteristicsW(
                windows::core::PCWSTR(class_name.as_ptr()),
                &mut task_index,
            );
            
            // 2. RAM Working Set 고정
            let process = windows::Win32::System::Threading::GetCurrentProcess();
            // 최소 500MB, 최대 2GB Working Set
            let min_size = 500 * 1024 * 1024;
            let max_size = 2000 * 1024 * 1024;
            let _ = windows::Win32::System::Memory::SetProcessWorkingSetSizeEx(
                process,
                min_size,
                max_size,
                windows::Win32::System::Memory::SETPROCESSWORKINGSETSIZEEX_FLAGS(0),
            );
        } else {
            // 일반 계정인 경우: 튕김(크래시) 없이 표준 프로세스로 Graceful Fallback
            println!("⚠️ 일반 사용자 계정으로 로그인되었습니다. 표준 스케줄링 모드로 구동합니다.");
        }
    }
}

impl AudioEngine {
    pub fn start(
        &mut self,
        device_name: Option<String>,
        cmd_receiver: crossbeam_channel::Receiver<AudioCommand>,
    ) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
            unsafe {
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            }
            apply_windows_admin_optimizations();
        }

        let target_prefix = device_name.as_ref().and_then(|n| {
            if n.starts_with("[ASIO]") {
                Some("[ASIO]")
            } else if n.starts_with("[WASAPI]") {
                Some("[WASAPI]")
            } else {
                None
            }
        });

        let hosts = get_hosts(target_prefix)?;

        let device = if let Some(ref name) = device_name {
            let mut found_device = None;
            let target_name = name
                .replace("[ASIO] ", "")
                .replace("[WASAPI] ", "")
                .replace("[CoreAudio] ", "");
            let target_name = target_name.replace('\0', "").trim().to_string();

            println!("🔥 [디버깅] 플러터 원본 요청: '{}'", name);
            println!("🔥 [디버깅] 공백 제거 후 타겟: '{}'", target_name);

            let start_time = std::time::Instant::now();
            let timeout_secs = 30;
            loop {
                for host in &hosts {
                    if let Ok(devices) = host.output_devices() {
                        for d in devices {
                            if let Ok(d_name) = d.name() {
                                let clean_d_name = d_name.replace('\0', "").trim().to_string();
                                if clean_d_name == target_name {
                                    found_device = Some(d);
                                    break;
                                }
                            }
                        }
                    }
                    if found_device.is_some() {
                        break;
                    }
                }

                if found_device.is_some() {
                    println!("✅ [디버깅] ASIO 오디오 인터페이스 인식 성공: {}", target_name);
                    break;
                }

                if start_time.elapsed().as_secs() >= timeout_secs {
                    break;
                }
                println!("⚠️ 장치를 찾는 중... ({} / 30초)", start_time.elapsed().as_secs());
                std::thread::sleep(std::time::Duration::from_millis(500));
            }

            if let Some(d) = found_device {
                d
            } else {
                let error_msg = format!("Requested device '{}' not found after 30s. Available devices were not matched.", name);
                eprintln!("{}", error_msg);
                return Err(error_msg);
            }
        } else {
            cpal::default_host()
                .default_output_device()
                .ok_or("No default output device".to_string())?
        };

        println!("Using output device: {}", device.name().unwrap_or_default());

        let default_config_result = device.default_output_config().ok();
        let mut best_config = default_config_result.clone();
        let mut max_ch = best_config.as_ref().map(|c| c.channels()).unwrap_or(0);
        let default_sample_rate = best_config
            .as_ref()
            .map(|c| c.sample_rate())
            .unwrap_or(cpal::SampleRate(48000));

        let mut supported_configs_result = device.supported_output_configs();
        for _ in 0..3 {
            if supported_configs_result.is_ok() {
                break;
            }
            println!("⏳ [ASIO Lock Retry] COM object might be busy. Waiting 500ms...");
            std::thread::sleep(std::time::Duration::from_millis(500));
            supported_configs_result = device.supported_output_configs();
        }

        if let Ok(supported_configs) = supported_configs_result {
            for c in supported_configs {
                if c.channels() > max_ch {
                    max_ch = c.channels();
                    if c.min_sample_rate() <= default_sample_rate
                        && c.max_sample_rate() >= default_sample_rate
                    {
                        best_config = Some(c.with_sample_rate(default_sample_rate));
                    } else {
                        best_config = Some(c.with_max_sample_rate());
                    }
                }
            }
        }

        let supported_config = best_config.expect("No output configs found");
        let sample_format = supported_config.sample_format();
        let mut config: StreamConfig = supported_config.clone().into();

        let mut target_buffer_size = 2048; // Force a safe, large default to prevent dropouts
        if let Some(app_config) = crate::core::state::GLOBAL_STATE
            .config
            .read()
            .unwrap()
            .as_ref()
        {
            if app_config.buffer_size > 0 {
                target_buffer_size = app_config.buffer_size;
            }
        }

        match supported_config.buffer_size() {
            cpal::SupportedBufferSize::Range { min, max } => {
                let clamped = target_buffer_size.clamp(*min, *max);
                config.buffer_size = cpal::BufferSize::Fixed(clamped);
                println!("🔥 [디버깅] Buffer size clamped to {} (Range: {} - {})", clamped, min, max);
            }
            cpal::SupportedBufferSize::Unknown => {
                config.buffer_size = cpal::BufferSize::Default;
                println!("⚠️ [디버깅] SupportedBufferSize::Unknown -> Using Default");
            }
        }

        config.sample_rate = supported_config.sample_rate();

        println!("Stream config: {:?}", config);

        crate::core::state::GLOBAL_STATE
            .active_device_channels
            .store(config.channels as u32, std::sync::atomic::Ordering::SeqCst);
        
        *crate::core::state::GLOBAL_STATE.engine_error.write().unwrap_or_else(|e| e.into_inner()) = None;

        let (gc_tx, gc_rx) =
            crossbeam_channel::bounded::<crate::audio::player::SoundInstance>(8192);
        std::thread::spawn(move || {
            while let Ok(dropped) = gc_rx.recv() {
                // Instance is dropped here in a background thread, preventing GC in audio thread.
                crate::core::state::GLOBAL_STATE.remove_playing_track(dropped.instance_id);
            }
        });

        let (analysis_tx, analysis_rx) = rtrb::RingBuffer::new(65536);
        crate::audio::analysis::start_analysis_thread(analysis_rx, config.sample_rate.0, config.channels as usize);

        let mut mixer = AudioMixer::new(config.sample_rate.0, config.channels as usize, gc_tx, Some(analysis_tx));

        let err_fn = |err: cpal::StreamError| {
            eprintln!("an error occurred on stream: {}", err);
            let is_disconnect = matches!(err, cpal::StreamError::DeviceNotAvailable);
            let msg = if is_disconnect {
                "DeviceNotAvailable".to_string()
            } else {
                err.to_string()
            };
            *crate::core::state::GLOBAL_STATE.engine_error.write().unwrap_or_else(|e| e.into_inner()) = Some(msg);
            crate::core::state::GLOBAL_STATE.broadcast_state();
        };

        let mut cmd_receiver_f32 = cmd_receiver;
        
        let stream = match sample_format {
            SampleFormat::F32 => device.build_output_stream(
                &config,
                move |data: &mut [f32], _: &OutputCallbackInfo| {
                    if !ENGINE_INIT_SIGNAL.load(Ordering::Acquire) {
                        ENGINE_INIT_SIGNAL.store(true, Ordering::Release);
                    }
                    Self::process_commands(&mut mixer, &mut cmd_receiver_f32);
                    mixer.process(data, config.channels as usize);
                },
                err_fn,
                None,
            ),
            SampleFormat::I16 => {
                let mut temp_buf: Vec<f32> = vec![0.0; 8192];
                device.build_output_stream(
                    &config,
                    move |data: &mut [i16], _: &OutputCallbackInfo| {
                        if !ENGINE_INIT_SIGNAL.load(Ordering::Acquire) {
                            ENGINE_INIT_SIGNAL.store(true, Ordering::Release);
                        }
                        Self::process_commands(&mut mixer, &mut cmd_receiver_f32);
                        if temp_buf.len() < data.len() {
                            temp_buf.resize(data.len(), 0.0);
                        }
                        let temp = &mut temp_buf[..data.len()];
                        mixer.process(temp, config.channels as usize);
                        for (dst, src) in data.iter_mut().zip(temp.iter()) {
                            *dst = cpal::Sample::from_sample(*src);
                        }
                    },
                    err_fn,
                    None,
                )
            }
            SampleFormat::I32 => {
                let mut temp_buf: Vec<f32> = vec![0.0; 8192];
                device.build_output_stream(
                    &config,
                    move |data: &mut [i32], _: &OutputCallbackInfo| {
                        if !ENGINE_INIT_SIGNAL.load(Ordering::Acquire) {
                            ENGINE_INIT_SIGNAL.store(true, Ordering::Release);
                        }
                        Self::process_commands(&mut mixer, &mut cmd_receiver_f32);
                        if temp_buf.len() < data.len() {
                            temp_buf.resize(data.len(), 0.0);
                        }
                        let temp = &mut temp_buf[..data.len()];
                        mixer.process(temp, config.channels as usize);
                        for (dst, src) in data.iter_mut().zip(temp.iter()) {
                            *dst = cpal::Sample::from_sample(*src);
                        }
                    },
                    err_fn,
                    None,
                )
            }
            SampleFormat::U16 => {
                let mut temp_buf: Vec<f32> = vec![0.0; 8192];
                device.build_output_stream(
                    &config,
                    move |data: &mut [u16], _: &OutputCallbackInfo| {
                        if !ENGINE_INIT_SIGNAL.load(Ordering::Acquire) {
                            ENGINE_INIT_SIGNAL.store(true, Ordering::Release);
                        }
                        Self::process_commands(&mut mixer, &mut cmd_receiver_f32);
                        if temp_buf.len() < data.len() {
                            temp_buf.resize(data.len(), 0.0);
                        }
                        let temp = &mut temp_buf[..data.len()];
                        mixer.process(temp, config.channels as usize);
                        for (dst, src) in data.iter_mut().zip(temp.iter()) {
                            *dst = cpal::Sample::from_sample(*src);
                        }
                    },
                    err_fn,
                    None,
                )
            }
            _ => return Err("Unsupported format".to_string()),
        };
        
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                // Task 3: 버퍼 사이즈 및 최대 채널 초과 시 폴백 처리
                eprintln!("Failed to build stream with config {:?}: {}", config, e);
                return Err(format!("Failed to build stream: {}", e));
            }
        };

        stream.play().map_err(|e| e.to_string())?;
        
        self.stream = Some(stream);
        
        Ok(())
    }

    fn process_commands(mixer: &mut AudioMixer, rx: &crossbeam_channel::Receiver<AudioCommand>) {
        // Lock-free pop from command queue
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                AudioCommand::PlayTrack {
                    instance_id,
                    room_id,
                    track_id,
                    track_id_str,
                    data,
                    stream_receiver,
                    stream_sample_rate,
                    stream_channels,
                    is_loop,
                    volume,
                    output_channel,
                    output_stereo,
                    current_position,
                } => {
                    let instance = crate::audio::player::SoundInstance::new(
                        instance_id,
                        track_id,
                        room_id,
                        track_id_str,
                        data,
                        stream_receiver,
                        stream_sample_rate,
                        stream_channels,
                        is_loop,
                        volume,
                        output_channel,
                        output_stereo,
                        current_position,
                    );
                    // instance.volume is already set in new
                    if let Some(slot) = mixer.instances.iter_mut().find(|s| s.is_none()) {
                        if let Some(old) = slot.replace(instance) {
                            let _ = mixer.gc_sender.try_send(old);
                        }
                    } else {
                        eprintln!("Mixer object pool full!");
                    }
                }
                AudioCommand::StopTrack { room_id, track_id } => {
                    for inst in mixer.instances.iter_mut().flatten() {
                        if inst.room_id == room_id && inst.id == track_id {
                            inst.is_stopping = true;
                        }
                    }
                }
                AudioCommand::StopAll => {
                    for inst in mixer.instances.iter_mut().flatten() {
                        inst.is_stopping = true;
                    }
                }
                AudioCommand::SetMasterMute { muted } => {
                    mixer.master_mute = muted;
                }
                AudioCommand::ClearRoom { room_id } => {
                    for inst in mixer.instances.iter_mut().flatten() {
                        if inst.room_id == room_id {
                            inst.is_stopping = true;
                        }
                    }
                }
                AudioCommand::SetMasterVolume { room_id, volume } => {
                    if let Some(slot) = mixer.room_volumes.iter_mut().find(|s| s.as_ref().is_some_and(|(id, _)| *id == room_id)) {
                        if let Some((_, v)) = slot.as_mut() {
                            *v = volume;
                        }
                    } else if let Some(empty_slot) = mixer.room_volumes.iter_mut().find(|s| s.is_none()) {
                        *empty_slot = Some((room_id, volume));
                    }
                }
                AudioCommand::SetTrackVolume {
                    room_id,
                    track_id,
                    volume,
                } => {
                    for inst in mixer.instances.iter_mut().flatten() {
                        if inst.room_id == room_id && inst.id == track_id {
                            inst.volume = volume;
                            inst.volume_smoother.set_target(volume);
                        }
                    }
                }
                AudioCommand::SetTrackOutput {
                    room_id,
                    track_id,
                    output_channel,
                    output_stereo,
                } => {
                    for inst in mixer.instances.iter_mut().flatten() {
                        if inst.room_id == room_id && inst.id == track_id {
                            inst.output_channel = output_channel;
                            inst.output_stereo = output_stereo;
                        }
                    }
                }
                AudioCommand::SetChannelDelay { channel, delay_ms } => {
                    if channel < mixer.channel_dsp.len() {
                        mixer.channel_dsp[channel].update_delay_target(delay_ms);
                    }
                }
                AudioCommand::SetChannelEq { channel, bands } => {
                    if channel < mixer.channel_dsp.len() {
                        mixer.channel_dsp[channel].update_eq_targets(&bands, mixer.sample_rate as f32);
                    }
                }
                AudioCommand::ApplyChannelTuning { channel, delay_ms, eq_bands } => {
                    if channel < mixer.channel_dsp.len() {
                        mixer.channel_dsp[channel].update_delay_target(delay_ms);
                        mixer.channel_dsp[channel].update_eq_targets(&eq_bands, mixer.sample_rate as f32);
                    }
                }
                AudioCommand::UpdateSpatialConfig { channel_positions, room_zones, trajectory, track_positions } => {
                    let old_positions = std::mem::replace(&mut mixer.channel_positions, channel_positions);
                    let old_zones = std::mem::replace(&mut mixer.room_zones, room_zones);
                    let old_traj = std::mem::replace(&mut mixer.trajectory, trajectory);
                    let _ = mixer.spatial_gc_tx.try_send(crate::audio::mixer::SpatialGarbage::ChannelPositions(old_positions));
                    let _ = mixer.spatial_gc_tx.try_send(crate::audio::mixer::SpatialGarbage::RoomZones(old_zones));
                    let _ = mixer.spatial_gc_tx.try_send(crate::audio::mixer::SpatialGarbage::Trajectory(old_traj));
                    
                    for inst in mixer.instances.iter_mut().flatten() {
                        if let Some(pos) = track_positions.get(&inst.track_id_str) {
                            inst.current_position = Some(pos.clone());
                        }
                    }
                    mixer.recalculate_spatial_dsp();
                }
                AudioCommand::UpdateTrajectoryPosition { position } => {
                    if let Some(traj) = &mut mixer.trajectory {
                        traj.current_position = position;
                    } else {
                        mixer.trajectory = Some(crate::common::config::Trajectory {
                            waypoints: vec![],
                            current_position: position,
                            ..Default::default()
                        });
                    }
                    mixer.recalculate_spatial_dsp();
                }
                AudioCommand::UpdateSingleBandEq { channel, band, freq, gain_db, q_factor, filter_type_idx } => {
                    if channel < mixer.channel_dsp.len() && band < mixer.channel_dsp[channel].target_bands.len() {
                        let filter_type = match filter_type_idx {
                            0 => crate::common::config::EqType::LowCut,
                            1 => crate::common::config::EqType::LowShelf,
                            2 => crate::common::config::EqType::Bell,
                            3 => crate::common::config::EqType::Notch,
                            4 => crate::common::config::EqType::HighShelf,
                            5 => crate::common::config::EqType::HighCut,
                            _ => crate::common::config::EqType::Bell,
                        };
                        let b = &mut mixer.channel_dsp[channel].target_bands[band];
                        b.freq = freq;
                        b.gain = gain_db;
                        b.q_factor = q_factor;
                        b.filter_type = filter_type;
                        // trigger recount/rebuild
                        let all_bands = mixer.channel_dsp[channel].target_bands.clone();
                        mixer.channel_dsp[channel].update_eq_targets(&all_bands, mixer.sample_rate as f32);
                    }
                }
                AudioCommand::UpdateSoundSourcePosition { sound_id, x, y, z } => {
                    let point = crate::common::config::Point3D { x, y, z, ..Default::default() };
                    // If it's the global trajectory ID we update it
                    if sound_id == "global_trajectory" || sound_id == "trajectory" {
                        if let Some(traj) = &mut mixer.trajectory {
                            traj.current_position = point;
                        } else {
                            mixer.trajectory = Some(crate::common::config::Trajectory {
                                waypoints: vec![],
                                current_position: point,
                                ..Default::default()
                            });
                        }
                    } else {
                        // Or if we map sound objects directly, we'd update their positions here.
                        // For now we'll support global trajectory.
                        if let Some(traj) = &mut mixer.trajectory {
                            traj.current_position = point;
                        }
                    }
                    mixer.recalculate_spatial_dsp();
                }
                AudioCommand::ApplyGlobalTuning { master_headroom_db, peak_limiter_enabled } => {
                    mixer.master_headroom_db = master_headroom_db;
                    mixer.peak_limiter_enabled = peak_limiter_enabled;
                }
                AudioCommand::ApplyAllChannelTunings { tunings } => {
                    for (channel, delay_ms, eq_bands) in tunings {
                        if channel < mixer.channel_dsp.len() {
                            mixer.channel_dsp[channel].update_delay_target(delay_ms);
                            mixer.channel_dsp[channel].update_eq_targets(&eq_bands, mixer.sample_rate as f32);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
