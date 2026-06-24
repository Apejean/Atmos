use crate::common::config::AppConfig;
use crate::core::state::GLOBAL_STATE;
use crate::common::commands::AudioCommand;
use crate::common::utils::hash_id;
use crate::api::error::AtmosError;
use crate::frb_generated::StreamSink;

#[flutter_rust_bridge::frb(init)]
pub fn api_init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

pub fn api_get_config(path: String) -> AppConfig {
    let config = AppConfig::load_from_file(path).unwrap_or_default();
    
    for b in &GLOBAL_STATE.enabled_channels {
        b.store(false, std::sync::atomic::Ordering::Relaxed);
    }
    if config.mono_configs.is_empty() && config.stereo_configs.is_empty() {
        for b in &GLOBAL_STATE.enabled_channels {
            b.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    } else {
        for (&ch, setting) in &config.mono_configs {
            if setting.enabled && ch > 0 {
                let real_ch = (ch - 1) as usize;
                if real_ch < GLOBAL_STATE.enabled_channels.len() {
                    GLOBAL_STATE.enabled_channels[real_ch].store(true, std::sync::atomic::Ordering::Relaxed);
                }
                if real_ch + 1 < GLOBAL_STATE.enabled_channels.len() {
                    GLOBAL_STATE.enabled_channels[real_ch + 1].store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
        for (&ch, setting) in &config.stereo_configs {
            if setting.enabled && ch > 0 {
                let real_ch = (ch - 1) as usize;
                if real_ch < GLOBAL_STATE.enabled_channels.len() {
                    GLOBAL_STATE.enabled_channels[real_ch].store(true, std::sync::atomic::Ordering::Relaxed);
                }
                if real_ch + 1 < GLOBAL_STATE.enabled_channels.len() {
                    GLOBAL_STATE.enabled_channels[real_ch + 1].store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
    }
    {
        let mut global_config = GLOBAL_STATE.config.write().unwrap();
        *global_config = Some(config.clone());
    }
    
    config
}

pub fn api_save_config(path: String, config: AppConfig) -> Result<(), AtmosError> {
    config.save_to_file(path)?;
    for b in &GLOBAL_STATE.enabled_channels {
        b.store(false, std::sync::atomic::Ordering::Relaxed);
    }
    if config.mono_configs.is_empty() && config.stereo_configs.is_empty() {
        for b in &GLOBAL_STATE.enabled_channels {
            b.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    } else {
        for (&ch, setting) in &config.mono_configs {
            if setting.enabled && ch > 0 {
                let real_ch = (ch - 1) as usize;
                if real_ch < GLOBAL_STATE.enabled_channels.len() {
                    GLOBAL_STATE.enabled_channels[real_ch].store(true, std::sync::atomic::Ordering::Relaxed);
                }
                if real_ch + 1 < GLOBAL_STATE.enabled_channels.len() {
                    GLOBAL_STATE.enabled_channels[real_ch + 1].store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
        for (&ch, setting) in &config.stereo_configs {
            if setting.enabled && ch > 0 {
                let real_ch = (ch - 1) as usize;
                if real_ch < GLOBAL_STATE.enabled_channels.len() {
                    GLOBAL_STATE.enabled_channels[real_ch].store(true, std::sync::atomic::Ordering::Relaxed);
                }
                if real_ch + 1 < GLOBAL_STATE.enabled_channels.len() {
                    GLOBAL_STATE.enabled_channels[real_ch + 1].store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
    }
    {
        let mut global_config = GLOBAL_STATE.config.write().unwrap();
        *global_config = Some(config);
    }
    Ok(())
}

pub fn api_play_track(room_id: String, track_id: String) -> Result<(), AtmosError> {
    let config_guard = GLOBAL_STATE.config.read().unwrap();
    if let Some(config) = config_guard.as_ref() {
        if let Some(room) = config.rooms.iter().find(|r| r.id == room_id) {
            if let Some(track) = room.tracks.iter().find(|t| t.id == track_id) {
                let instance_id = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64;
                if track.is_loop {
                    // Prevent duplicate playback of the same looping track
                    let is_playing = {
                        let guard = GLOBAL_STATE.playing_track_ids.read().unwrap();
                        guard.values().any(|id| id == &track_id)
                    };
                    if is_playing {
                        return Ok(());
                    }

                    // Start DiskStreamer for BGM
                    match crate::audio::streaming::DiskStreamer::new(track.file_path.clone()) {
                        Ok(streamer) => {
                            GLOBAL_STATE.add_playing_track(instance_id, track_id.clone());
                            GLOBAL_STATE.command_sender.try_send(AudioCommand::PlayTrack {
                                instance_id,
                                room_id: hash_id(&room_id),
                                track_id: hash_id(&track_id),
                                track_id_str: track_id.clone(),
                                data: None,
                                stream_receiver: Some(streamer.chunk_receiver),
                                stream_sample_rate: streamer.sample_rate,
                                stream_channels: streamer.channels,
                                is_loop: true,
                                volume: track.volume,
                                output_channel: track.output_channel as usize,
                                output_stereo: track.output_stereo,
                            }).map_err(|e| AtmosError { message: e.to_string() })?;
                            return Ok(());
                        }
                        Err(e) => {
                            return Err(AtmosError { message: format!("Streamer init failed: {}", e) });
                        }
                    }
                } else {
                    let cache_guard = GLOBAL_STATE.sound_cache.read().unwrap();
                    if let Some(data) = cache_guard.get(&track.file_path) {
                        GLOBAL_STATE.add_playing_track(instance_id, track_id.clone());
                        GLOBAL_STATE.command_sender.try_send(AudioCommand::PlayTrack {
                            instance_id,
                            room_id: hash_id(&room_id),
                            track_id: hash_id(&track_id),
                            track_id_str: track_id.clone(),
                            data: Some(data.clone()),
                            stream_receiver: None,
                            stream_sample_rate: data.sample_rate,
                            stream_channels: data.channels,
                            is_loop: false,
                            volume: track.volume,
                            output_channel: track.output_channel as usize,
                            output_stereo: track.output_stereo,
                        }).map_err(|e| AtmosError { message: e.to_string() })?;
                        return Ok(());
                    } else {
                        return Err(AtmosError { message: format!("Cache miss for {}", track.file_path) });
                    }
                }
            }
        }
    }
    Err(AtmosError { message: "Room or track not found".to_string() })
}

pub fn api_stop_track(room_id: String, track_id: String) -> Result<(), AtmosError> {
    GLOBAL_STATE.remove_playing_tracks_by_track_id(&track_id);
    GLOBAL_STATE.command_sender.try_send(AudioCommand::StopTrack { 
        room_id: hash_id(&room_id), 
        track_id: hash_id(&track_id) 
    }).map_err(|e| AtmosError { message: e.to_string() })?;
    Ok(())
}

pub fn api_stop_all() -> Result<(), AtmosError> {
    let _lock = GLOBAL_STATE.broadcast_lock.lock().unwrap();
    {
        let mut guard = GLOBAL_STATE.playing_track_ids.write().unwrap();
        guard.clear();
    }
    {
        let mut guard = GLOBAL_STATE.active_room_id.write().unwrap();
        *guard = None;
    }
    GLOBAL_STATE.broadcast_state();
    GLOBAL_STATE.command_sender.try_send(AudioCommand::StopAll)
        .map_err(|e| AtmosError { message: e.to_string() })?;
    Ok(())
}

pub fn api_set_active_room(room_id: Option<String>) -> Result<(), AtmosError> {
    GLOBAL_STATE.set_active_room(room_id);
    Ok(())
}

pub fn api_clear_room(room_id: String) -> Result<(), AtmosError> {
    // When a room is cleared, we might want to just clear playing tracks, but usually it stops them too.
    let _lock = GLOBAL_STATE.broadcast_lock.lock().unwrap();
    {
        let mut guard = GLOBAL_STATE.active_room_id.write().unwrap();
        if guard.as_ref() != Some(&room_id) {
            return Err(AtmosError { message: "Room is not active or already cleared".to_string() });
        }
        *guard = None;
    }
    {
        let mut guard = GLOBAL_STATE.playing_track_ids.write().unwrap();
        guard.clear();
    }
    GLOBAL_STATE.broadcast_state();
    GLOBAL_STATE.command_sender.try_send(AudioCommand::ClearRoom { room_id: hash_id(&room_id) })
        .map_err(|e| AtmosError { message: e.to_string() })?;
    Ok(())
}

pub fn api_set_master_volume(room_id: String, volume: f32) -> Result<(), AtmosError> {
    GLOBAL_STATE.command_sender.try_send(AudioCommand::SetMasterVolume { room_id: hash_id(&room_id), volume })
        .map_err(|e| AtmosError { message: e.to_string() })?;
    Ok(())
}

pub fn api_set_track_volume(room_id: String, track_id: String, volume: f32) -> Result<(), AtmosError> {
    GLOBAL_STATE.command_sender.try_send(AudioCommand::SetTrackVolume { room_id: hash_id(&room_id), track_id: hash_id(&track_id), volume })
        .map_err(|e| AtmosError { message: e.to_string() })?;
    Ok(())
}

use std::sync::atomic::AtomicU64;
lazy_static::lazy_static! {
    static ref VU_THREAD_RUNNING: AtomicU64 = AtomicU64::new(0);
}

pub fn api_create_vu_stream(sink: StreamSink<Vec<f32>>) {
    let session_id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
    VU_THREAD_RUNNING.store(session_id, std::sync::atomic::Ordering::Relaxed);
    
    std::thread::spawn(move || {
        loop {
            if VU_THREAD_RUNNING.load(std::sync::atomic::Ordering::Relaxed) != session_id {
                break;
            }
            let levels: Vec<f32> = GLOBAL_STATE.vu_levels.iter().map(|v| f32::from_bits(v.load(std::sync::atomic::Ordering::Relaxed))).collect();
            let _ = sink.add(levels);
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    });
}

lazy_static::lazy_static! {
    static ref ENGINE_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    static ref ENGINE_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
}

pub fn api_start_audio_engine(device_name: Option<String>) {
    let rx = GLOBAL_STATE.command_receiver.clone();
    let gen = ENGINE_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    
    std::thread::spawn(move || {
        // Wait for previous engine to fully drop to release ASIO locks
        for _ in 0..60 {
            if !ENGINE_ACTIVE.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        
        // Give ASIO driver extra time to fully release hardware locks
        std::thread::sleep(std::time::Duration::from_millis(500));
        
        ENGINE_ACTIVE.store(true, std::sync::atomic::Ordering::SeqCst);

        #[cfg(target_os = "windows")]
        {
            use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
            unsafe {
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            }
        }
        
        let mut engine = crate::audio::engine::AudioEngine::new();
        if let Err(e) = engine.start(device_name, rx) {
            let err_msg = format!("Failed to start audio engine: {}", e);
            eprintln!("{}", err_msg);
            GLOBAL_STATE.log(err_msg);
            ENGINE_ACTIVE.store(false, std::sync::atomic::Ordering::SeqCst);
            return;
        }
        
        loop { 
            if ENGINE_GENERATION.load(std::sync::atomic::Ordering::SeqCst) != gen {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100)); 
        }
        
        drop(engine);
        ENGINE_ACTIVE.store(false, std::sync::atomic::Ordering::SeqCst);
    });
}

pub fn api_stop_audio_engine() {
    let _ = ENGINE_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    for _ in 0..40 {
        if !ENGINE_ACTIVE.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    println!("✅ [디버깅] 백엔드 오디오 엔진 명시적 종료 완료.");
}

pub fn api_start_osc_listener(port: u16) {
    let listener = crate::osc::listener::OscListener::new();
    listener.start(port);
}

pub fn api_create_log_stream(sink: StreamSink<String>) {
    let _ = sink.add("Rust Engine Log Stream Connected".to_string());
    *GLOBAL_STATE.log_sink.write().unwrap() = Some(sink);
}

#[derive(Debug, Clone)]
pub struct EngineStateUpdate {
    pub active_room_id: Option<String>,
    pub ducking_active: bool,
    pub playing_track_ids: Vec<String>,
}

pub fn api_create_engine_state_stream(sink: StreamSink<EngineStateUpdate>) {
    let playing_track_ids = {
        let guard = GLOBAL_STATE.playing_track_ids.read().unwrap();
        let mut unique_ids: Vec<String> = guard.values().cloned().collect();
        unique_ids.sort();
        unique_ids.dedup();
        unique_ids
    };
    
    let initial_state = EngineStateUpdate {
        active_room_id: GLOBAL_STATE.active_room_id.read().unwrap().clone(),
        ducking_active: GLOBAL_STATE.is_ducking.load(std::sync::atomic::Ordering::Relaxed),
        playing_track_ids,
    };
    let _ = sink.add(initial_state);
    *GLOBAL_STATE.state_sink.write().unwrap() = Some(sink);
}

pub fn api_preload_all_sounds(config: AppConfig) -> Result<(), AtmosError> {
    let mut cache = GLOBAL_STATE.sound_cache.write().unwrap();
    cache.clear();
    let mut errors = Vec::new();
    for room in &config.rooms {
        for track in &room.tracks {
            // Only preload SFX (not loops) to save RAM
            if !track.is_loop && !cache.contains_key(&track.file_path) {
                let path = std::path::Path::new(&track.file_path);
                match crate::audio::player::SoundData::load_from_file(path) {
                    Ok(data) => {
                        GLOBAL_STATE.log(format!("Loaded sound file: {}", track.file_path));
                        cache.insert(track.file_path.clone(), std::sync::Arc::new(data));
                    }
                    Err(e) => {
                        let err_msg = format!("Failed to load sound file {}: {}", track.file_path, e);
                        GLOBAL_STATE.log(err_msg.clone());
                        errors.push(err_msg);
                    }
                }
            }
        }
    }
    
    for b in &GLOBAL_STATE.enabled_channels {
        b.store(false, std::sync::atomic::Ordering::Relaxed);
    }
    if config.mono_configs.is_empty() && config.stereo_configs.is_empty() {
        for b in &GLOBAL_STATE.enabled_channels {
            b.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    } else {
        for (&ch, setting) in &config.mono_configs {
            if setting.enabled && ch > 0 {
                let real_ch = (ch - 1) as usize;
                if real_ch < GLOBAL_STATE.enabled_channels.len() {
                    GLOBAL_STATE.enabled_channels[real_ch].store(true, std::sync::atomic::Ordering::Relaxed);
                }
                if real_ch + 1 < GLOBAL_STATE.enabled_channels.len() {
                    GLOBAL_STATE.enabled_channels[real_ch + 1].store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
        for (&ch, setting) in &config.stereo_configs {
            if setting.enabled && ch > 0 {
                let real_ch = (ch - 1) as usize;
                if real_ch < GLOBAL_STATE.enabled_channels.len() {
                    GLOBAL_STATE.enabled_channels[real_ch].store(true, std::sync::atomic::Ordering::Relaxed);
                }
                if real_ch + 1 < GLOBAL_STATE.enabled_channels.len() {
                    GLOBAL_STATE.enabled_channels[real_ch + 1].store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
    }

    let mut global_config = GLOBAL_STATE.config.write().unwrap();
    *global_config = Some(config.clone());
    drop(global_config);
    
    // Check if active_room_id exists in the new config
    let active_room_id = {
        let guard = GLOBAL_STATE.active_room_id.read().unwrap();
        guard.clone()
    };
    
    if let Some(active_id) = active_room_id {
        let room_exists = config.rooms.iter().any(|r| r.id == active_id);
        if !room_exists {
            // Room was deleted! Safely clear the room
            let _ = api_clear_room(active_id);
        }
    }
    
    if !errors.is_empty() {
        return Err(AtmosError { message: errors.join(" | ") });
    }
    
    Ok(())
}

#[derive(Clone, Debug)]
pub struct OutputDeviceInfo {
    pub name: String,
    pub max_channels: u32,
    pub channel_names: Vec<String>,
}

pub fn api_get_output_devices() -> Result<Vec<OutputDeviceInfo>, AtmosError> {
    std::thread::spawn(|| {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
            unsafe {
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            }
        }
        
        use cpal::traits::{DeviceTrait, HostTrait};
        
        let is_engine_active = ENGINE_ACTIVE.load(std::sync::atomic::Ordering::SeqCst);
        let mut skip_asio_scan = false;
        let mut active_asio_device = None;
        
        if is_engine_active {
            if let Some(config) = crate::core::state::GLOBAL_STATE.config.read().unwrap().as_ref() {
                if let Some(ref saved_name) = config.device_name {
                    if saved_name.starts_with("[ASIO]") {
                        skip_asio_scan = true;
                        active_asio_device = Some(saved_name.clone());
                    }
                }
            }
        }

        let target_prefix = if skip_asio_scan { Some("[WASAPI]") } else { None };
        let hosts = crate::audio::engine::get_hosts(target_prefix)
            .map_err(|e| AtmosError { message: e })?;
        
        let mut device_info_list = Vec::new();
        
        if let Some(saved_name) = active_asio_device {
            let max_channels = 2; // Fallback
            let actual_name = saved_name.replace("[ASIO] ", "").trim().to_string();
            #[cfg(target_os = "macos")]
            let channel_names = crate::audio::channel_names::get_channel_names_mac(&actual_name, max_channels);
            #[cfg(target_os = "windows")]
            let channel_names = crate::audio::channel_names::get_channel_names_win(&actual_name, max_channels);
            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            let channel_names = crate::audio::channel_names::get_channel_names_fallback(max_channels);

            device_info_list.push(OutputDeviceInfo { name: saved_name, max_channels, channel_names });
        }
        
        for host in hosts {
            let prefix = format!("[{}] ", host.id().name());
            let devices = match host.output_devices() {
                Ok(d) => d,
                Err(e) => { 
                    eprintln!("Failed to get output devices for host {}: {:?}", host.id().name(), e);
                    // If ASIO is locked, we can fallback to checking if the saved device was ASIO
                    if host.id().name() == "ASIO" {
                        if let Some(config) = crate::core::state::GLOBAL_STATE.config.read().unwrap().as_ref() {
                            if let Some(ref saved_name) = config.device_name {
                                if saved_name.starts_with("[ASIO]") {
                                    let max_channels = 2; // Fallback
                                    let actual_name = saved_name.replace("[ASIO] ", "").trim().to_string();
                                    #[cfg(target_os = "macos")]
                                    let channel_names = crate::audio::channel_names::get_channel_names_mac(&actual_name, max_channels);
                                    #[cfg(target_os = "windows")]
                                    let channel_names = crate::audio::channel_names::get_channel_names_win(&actual_name, max_channels);
                                    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                                    let channel_names = crate::audio::channel_names::get_channel_names_fallback(max_channels);

                                    device_info_list.push(OutputDeviceInfo { name: saved_name.clone(), max_channels, channel_names });
                                }
                            }
                        }
                    }
                    continue; 
                },
            };
            
            for device in devices {
                if let Ok(name_str) = device.name() {
                    let actual_name = name_str.replace('\0', "").trim().to_string();
                    let name = format!("{}{}", prefix, actual_name);
                    let mut max_channels = 2; // Default fallback
                    if let Ok(supported_configs) = device.supported_output_configs() {
                        for config in supported_configs {
                            let channels = config.channels() as u32;
                            if channels > max_channels {
                                max_channels = channels;
                            }
                        }
                    }
                    // ASIO fallback for max channels
                    if let Ok(default_config) = device.default_output_config() {
                        let channels = default_config.channels() as u32;
                        if channels > max_channels {
                            max_channels = channels;
                        }
                    }
                    
                    #[cfg(target_os = "macos")]
                    let channel_names = crate::audio::channel_names::get_channel_names_mac(&actual_name, max_channels);
                    #[cfg(target_os = "windows")]
                    let channel_names = crate::audio::channel_names::get_channel_names_win(&actual_name, max_channels);
                    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                    let channel_names = crate::audio::channel_names::get_channel_names_fallback(max_channels);

                    device_info_list.push(OutputDeviceInfo { name, max_channels, channel_names });
                }
            }
        }
        
        Ok(device_info_list)
    }).join().unwrap_or_else(|_| Err(AtmosError { message: "Thread panicked during device lookup".to_string() }))
}

pub fn api_get_device_channel_count(device_name: Option<String>) -> Result<u32, AtmosError> {
    std::thread::spawn(move || {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
            unsafe {
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            }
        }
        use cpal::traits::{DeviceTrait, HostTrait};
        
        let is_engine_active = ENGINE_ACTIVE.load(std::sync::atomic::Ordering::SeqCst);
        let mut active_asio_device = None;
        if is_engine_active {
            if let Some(config) = crate::core::state::GLOBAL_STATE.config.read().unwrap().as_ref() {
                if let Some(ref saved_name) = config.device_name {
                    if saved_name.starts_with("[ASIO]") {
                        active_asio_device = Some(saved_name.clone());
                    }
                }
            }
        }

        let device = if let Some(ref name) = device_name {
            if let Some(ref active_name) = active_asio_device {
                if name == active_name {
                    return Ok(2); // Fallback to 2 channels to avoid querying locked ASIO device
                }
            }
            
            let target_prefix = if name.starts_with("[ASIO]") { Some("[ASIO]") } 
                else if name.starts_with("[WASAPI]") { Some("[WASAPI]") } 
                else if name.starts_with("[CoreAudio]") { Some("[CoreAudio]") }
                else { None };
                
            let hosts = crate::audio::engine::get_hosts(target_prefix)
                .map_err(|e| AtmosError { message: e })?;
                
            let mut found_device = None;
            let target_name = name.replace("[ASIO] ", "").replace("[WASAPI] ", "").replace("[CoreAudio] ", "");
            let target_name = target_name.replace('\0', "").trim().to_string();

            for host in &hosts {
                if let Ok(devices) = host.output_devices() {
                    for d in devices {
                        if let Ok(d_name) = d.name() {
                            if d_name.replace('\0', "").trim() == target_name {
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
            found_device.ok_or_else(|| AtmosError { message: format!("Device not found: {}", name) })?
        } else {
            cpal::default_host().default_output_device().ok_or_else(|| AtmosError { message: "No default output device".to_string() })?
        };

        let mut max_channels = 0;
        if let Ok(supported_configs) = device.supported_output_configs() {
            for config in supported_configs {
                let channels = config.channels() as u32;
                if channels > max_channels {
                    max_channels = channels;
                }
            }
        }
        
        if let Ok(default_config) = device.default_output_config() {
            let channels = default_config.channels() as u32;
            if channels > max_channels {
                max_channels = channels;
            }
        }

        Ok(max_channels)
    }).join().unwrap_or_else(|_| Err(AtmosError { message: "Thread panicked during channel count".to_string() }))
}

pub fn api_get_device_channel_names(device_name: Option<String>) -> Result<Vec<String>, AtmosError> {
    let max_channels = api_get_device_channel_count(device_name.clone())?;
    
    let actual_name = if let Some(ref name) = device_name {
        if let Some(idx) = name.find("] ") {
            name[idx + 2..].to_string()
        } else {
            name.clone()
        }
    } else {
        "Default".to_string()
    };

    #[cfg(target_os = "macos")]
    let channel_names = crate::audio::channel_names::get_channel_names_mac(&actual_name, max_channels);
    #[cfg(target_os = "windows")]
    let channel_names = crate::audio::channel_names::get_channel_names_win(&actual_name, max_channels);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let channel_names = crate::audio::channel_names::get_channel_names_fallback(max_channels);
    
    Ok(channel_names)
}
