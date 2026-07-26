use crate::api::error::AtmosError;
use crate::common::commands::AudioCommand;
use crate::common::config::AppConfig;
use crate::common::utils::hash_id;
use crate::core::state::GLOBAL_STATE;
use crate::frb_generated::StreamSink;

#[flutter_rust_bridge::frb(init)]
pub fn api_init_app() {
    flutter_rust_bridge::setup_default_user_utils();
    
    // Start OSC server in background
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            crate::api::osc::start_osc_server().await;
        });
    });
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
                    GLOBAL_STATE.enabled_channels[real_ch]
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
                if real_ch + 1 < GLOBAL_STATE.enabled_channels.len() {
                    GLOBAL_STATE.enabled_channels[real_ch + 1]
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
        for (&ch, setting) in &config.stereo_configs {
            if setting.enabled && ch > 0 {
                let real_ch = (ch - 1) as usize;
                if real_ch < GLOBAL_STATE.enabled_channels.len() {
                    GLOBAL_STATE.enabled_channels[real_ch]
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
                if real_ch + 1 < GLOBAL_STATE.enabled_channels.len() {
                    GLOBAL_STATE.enabled_channels[real_ch + 1]
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
    }
    {
        let mut global_config = GLOBAL_STATE.config.write().unwrap_or_else(|e| e.into_inner());
        *global_config = Some(config.clone());
    }
    GLOBAL_STATE.is_exhibition_mode.store(config.is_exhibition_mode, std::sync::atomic::Ordering::Relaxed);

    // Apply tunings on boot immediately
    let mut tunings = Vec::new();
    for (&ch, setting) in &config.mono_configs {
        if setting.enabled {
            tunings.push((ch as usize - 1, setting.delay_ms, setting.eq_bands.clone()));
        }
    }
    for (&ch, setting) in &config.stereo_configs {
        if setting.enabled {
            tunings.push((ch as usize - 1, setting.delay_ms, setting.eq_bands.clone()));
        }
    }
    let _ = GLOBAL_STATE.command_sender.try_send(AudioCommand::ApplyAllChannelTunings { tunings });

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
                    GLOBAL_STATE.enabled_channels[real_ch]
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
                if real_ch + 1 < GLOBAL_STATE.enabled_channels.len() {
                    GLOBAL_STATE.enabled_channels[real_ch + 1]
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
        for (&ch, setting) in &config.stereo_configs {
            if setting.enabled && ch > 0 {
                let real_ch = (ch - 1) as usize;
                if real_ch < GLOBAL_STATE.enabled_channels.len() {
                    GLOBAL_STATE.enabled_channels[real_ch]
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
                if real_ch + 1 < GLOBAL_STATE.enabled_channels.len() {
                    GLOBAL_STATE.enabled_channels[real_ch + 1]
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
    }
    {
        let mut global_config = GLOBAL_STATE.config.write().unwrap_or_else(|e| e.into_inner());
        *global_config = Some(config.clone());
    }
    GLOBAL_STATE.is_exhibition_mode.store(config.is_exhibition_mode, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

pub fn api_play_test_noise(channel: u32) -> Result<(), AtmosError> {
    GLOBAL_STATE
        .command_sender
        .try_send(AudioCommand::PlayTestNoise { channel })
        .map_err(|e| AtmosError {
            message: e.to_string(),
        })?;
    Ok(())
}

pub fn api_preload_sound(file_path: String) -> Result<(), AtmosError> {
    let path = std::path::Path::new(&file_path);
    match crate::audio::player::SoundData::load_from_file(path) {
        Ok(data) => {
            // 100MB limit check (samples * 4 bytes per f32)
            if data.samples.len() > 25_000_000 {
                println!("[TELEMETRY] OOM Protection triggered: File {} exceeds 100MB limit.", file_path);
                return Err(AtmosError {
                    message: "File is too large for preload (exceeds 100MB).".to_string(),
                });
            }
            let arc_data = std::sync::Arc::new(data);
            let mut preloaded = GLOBAL_STATE.preloaded_sounds.write().unwrap_or_else(|e| e.into_inner());
            preloaded.insert(file_path, arc_data);
            Ok(())
        }
        Err(e) => Err(AtmosError {
            message: format!("Preload failed: {}", e),
        }),
    }
}

pub fn api_clear_preloaded_sounds() -> Result<(), AtmosError> {
    let mut preloaded = GLOBAL_STATE.preloaded_sounds.write().unwrap_or_else(|e| e.into_inner());
    preloaded.clear();
    Ok(())
}

pub fn api_play_track(room_id: String, track_id: String) -> Result<(), AtmosError> {
    let config_guard = GLOBAL_STATE.config.read().unwrap_or_else(|e| e.into_inner());
    if let Some(config) = config_guard.as_ref() {
        if let Some(room) = config.rooms.iter().find(|r| r.id == room_id) {
            if let Some(track) = room.tracks.iter().find(|t| t.id == track_id) {
                let instance_id = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64;

                let _ = GLOBAL_STATE
                    .command_sender
                    .try_send(AudioCommand::SetMasterVolume {
                        room_id: hash_id(&room_id),
                        volume: room.volume,
                    });
                if track.is_loop || track.is_streaming {
                    // Prevent duplicate playback of the same looping track if it is a BGM
                    if track.is_loop {
                        let is_playing = {
                            let guard = GLOBAL_STATE.playing_track_ids.read().unwrap_or_else(|e| e.into_inner());
                            guard.values().any(|id| id == &track_id)
                        };
                        if is_playing {
                            return Ok(());
                        }
                    }

                    // Start DiskStreamer for BGM or streaming tracks
                    match crate::audio::streaming::DiskStreamer::new(track.file_path.clone(), track.is_loop) {
                        Ok(streamer) => {
                            GLOBAL_STATE.add_playing_track(instance_id, track_id.clone());
                            GLOBAL_STATE
                                .command_sender
                                .try_send(AudioCommand::PlayTrack {
                                    instance_id,
                                    room_id: hash_id(&room_id),
                                    track_id: hash_id(&track_id),
                                    track_id_str: track_id.clone(),
                                    data: None,
                                    stream_receiver: Some(streamer.chunk_receiver),
                                    stream_sample_rate: streamer.sample_rate,
                                    stream_channels: streamer.channels,
                                    is_loop: track.is_loop,
                                    volume: track.volume,
                                    output_channel: track.output_channel as usize,
                                    output_stereo: track.output_stereo,
                                })
                                .map_err(|e| AtmosError {
                                    message: e.to_string(),
                                })?;
                            return Ok(());
                        }
                        Err(e) => {
                            return Err(AtmosError {
                                message: format!("Streamer init failed: {}", e),
                            });
                        }
                    }
                } else {
                    let preloaded_data = {
                        let preloaded_guard = GLOBAL_STATE.preloaded_sounds.read().unwrap_or_else(|e| e.into_inner());
                        preloaded_guard.get(&track.file_path).cloned()
                    };

                    if let Some(data) = preloaded_data {
                        // Protect against multiple rapid clicks for looping tracks
                        if track.is_loop {
                            let is_playing = {
                                let guard = GLOBAL_STATE.playing_track_ids.read().unwrap_or_else(|e| e.into_inner());
                                guard.values().any(|id| id == &track_id)
                            };
                            if is_playing {
                                return Ok(());
                            }
                        }

                        GLOBAL_STATE.add_playing_track(instance_id, track_id.clone());
                        GLOBAL_STATE
                            .command_sender
                            .try_send(AudioCommand::PlayTrack {
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
                            })
                            .map_err(|e| AtmosError {
                                message: e.to_string(),
                            })?;
                        return Ok(());
                    }

                    let cache_guard = GLOBAL_STATE.sound_cache.read().unwrap_or_else(|e| e.into_inner());
                    if let Some(data) = cache_guard.get(&track.file_path) {
                        GLOBAL_STATE.add_playing_track(instance_id, track_id.clone());
                        GLOBAL_STATE
                            .command_sender
                            .try_send(AudioCommand::PlayTrack {
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
                            })
                            .map_err(|e| AtmosError {
                                message: e.to_string(),
                            })?;
                        return Ok(());
                    } else {
                        // Cache miss -> Load into RAM dynamically (obeys 100% RAM rule for SFX)
                        let path = std::path::Path::new(&track.file_path);
                        if let Ok(metadata) = std::fs::metadata(path) {
                            if metadata.len() > 500 * 1024 * 1024 {
                                return Err(AtmosError {
                                    message: format!("파일 용량이 너무 큽니다 (500MB 초과). BGM(Loop)으로 설정하거나 용량을 줄이세요: {}", track.file_path),
                                });
                            }
                        }
                        match crate::audio::player::SoundData::load_from_file(path) {
                            Ok(data) => {
                                let arc_data = std::sync::Arc::new(data);
                                {
                                    let mut cache = GLOBAL_STATE.sound_cache.write().unwrap_or_else(|e| e.into_inner());
                                    cache.insert(track.file_path.clone(), arc_data.clone());
                                }
                                GLOBAL_STATE.add_playing_track(instance_id, track_id.clone());
                                GLOBAL_STATE
                                    .command_sender
                                    .try_send(AudioCommand::PlayTrack {
                                        instance_id,
                                        room_id: hash_id(&room_id),
                                        track_id: hash_id(&track_id),
                                        track_id_str: track_id.clone(),
                                        data: Some(arc_data.clone()),
                                        stream_receiver: None,
                                        stream_sample_rate: arc_data.sample_rate,
                                        stream_channels: arc_data.channels,
                                        is_loop: false,
                                        volume: track.volume,
                                        output_channel: track.output_channel as usize,
                                        output_stereo: track.output_stereo,
                                    })
                                    .map_err(|e| AtmosError {
                                        message: e.to_string(),
                                    })?;
                                return Ok(());
                            }
                            Err(e) => {
                                return Err(AtmosError {
                                    message: format!("Cache miss and dynamic RAM loading failed for {}: {}", track.file_path, e),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    Err(AtmosError {
        message: "Room or track not found".to_string(),
    })
}

pub fn api_stop_track(room_id: String, track_id: String) -> Result<(), AtmosError> {
    GLOBAL_STATE.remove_playing_tracks_by_track_id(&track_id);
    GLOBAL_STATE
        .command_sender
        .try_send(AudioCommand::StopTrack {
            room_id: hash_id(&room_id),
            track_id: hash_id(&track_id),
        })
        .map_err(|e| AtmosError {
            message: e.to_string(),
        })?;
    Ok(())
}

pub fn api_stop_all() -> Result<(), AtmosError> {
    let _lock = GLOBAL_STATE.broadcast_lock.lock().unwrap_or_else(|e| e.into_inner());
    {
        let mut guard = GLOBAL_STATE.playing_track_ids.write().unwrap_or_else(|e| e.into_inner());
        guard.clear();
    }
    {
        let mut guard = GLOBAL_STATE.active_room_id.write().unwrap_or_else(|e| e.into_inner());
        *guard = None;
    }
    GLOBAL_STATE.broadcast_state();
    GLOBAL_STATE
        .command_sender
        .try_send(AudioCommand::StopAll)
        .map_err(|e| AtmosError {
            message: e.to_string(),
        })?;
    Ok(())
}

pub fn api_set_active_room(room_id: Option<String>) -> Result<(), AtmosError> {
    GLOBAL_STATE.set_active_room(room_id);
    Ok(())
}

pub fn api_clear_room(room_id: String) -> Result<(), AtmosError> {
    // Acquire locks in a strict hierarchy to prevent deadlocks:
    // 1. active_room_id
    // 2. playing_track_ids
    // 3. broadcast_lock (held ONLY during the channel send, not wrapping others)
    {
        let mut guard = GLOBAL_STATE.active_room_id.write().unwrap_or_else(|e| e.into_inner());
        if guard.as_ref() != Some(&room_id) {
            return Err(AtmosError {
                message: "Room is not active or already cleared".to_string(),
            });
        }
        *guard = None;
    }
    {
        let mut guard = GLOBAL_STATE.playing_track_ids.write().unwrap_or_else(|e| e.into_inner());
        guard.clear();
    }
    GLOBAL_STATE.broadcast_state();

    let _lock = GLOBAL_STATE.broadcast_lock.lock().unwrap_or_else(|e| e.into_inner());
    GLOBAL_STATE
        .command_sender
        .try_send(AudioCommand::ClearRoom {
            room_id: hash_id(&room_id),
        })
        .map_err(|e| AtmosError {
            message: e.to_string(),
        })?;
    Ok(())
}

pub fn api_set_master_mute(muted: bool) -> Result<(), AtmosError> {
    GLOBAL_STATE
        .command_sender
        .try_send(AudioCommand::SetMasterMute {
            muted,
        })
        .map_err(|e| AtmosError {
            message: e.to_string(),
        })?;
    Ok(())
}

pub fn api_set_master_volume(room_id: String, volume: f32) -> Result<(), AtmosError> {
    GLOBAL_STATE
        .command_sender
        .try_send(AudioCommand::SetMasterVolume {
            room_id: hash_id(&room_id),
            volume,
        })
        .map_err(|e| AtmosError {
            message: e.to_string(),
        })?;
    Ok(())
}

#[flutter_rust_bridge::frb(sync)]
pub fn api_set_track_volume(
    room_id: String,
    track_id: String,
    volume: f32,
) -> Result<(), AtmosError> {
    GLOBAL_STATE
        .command_sender
        .try_send(AudioCommand::SetTrackVolume {
            room_id: hash_id(&room_id),
            track_id: hash_id(&track_id),
            volume,
        })
        .map_err(|e| AtmosError {
            message: e.to_string(),
        })?;
    Ok(())
}

pub fn api_set_track_output(
    room_id: String,
    track_id: String,
    output_channel: usize,
    output_stereo: bool,
) -> Result<(), AtmosError> {
    GLOBAL_STATE
        .command_sender
        .try_send(AudioCommand::SetTrackOutput {
            room_id: hash_id(&room_id),
            track_id: hash_id(&track_id),
            output_channel,
            output_stereo,
        })
        .map_err(|e| AtmosError {
            message: e.to_string(),
        })?;
    Ok(())
}

pub fn api_set_channel_delay(channel: usize, delay_ms: f32) -> Result<(), AtmosError> {
    GLOBAL_STATE
        .command_sender
        .try_send(AudioCommand::SetChannelDelay {
            channel,
            delay_ms,
        })
        .map_err(|e| AtmosError {
            message: e.to_string(),
        })?;
    Ok(())
}

pub fn api_set_channel_eq(channel: usize, bands: Vec<crate::common::config::EqBand>) -> Result<(), AtmosError> {
    GLOBAL_STATE
        .command_sender
        .try_send(AudioCommand::SetChannelEq {
            channel,
            bands,
        })
        .map_err(|e| AtmosError {
            message: e.to_string(),
        })?;
    Ok(())
}

use std::sync::atomic::AtomicU64;
lazy_static::lazy_static! {
    static ref VU_THREAD_RUNNING: AtomicU64 = AtomicU64::new(0);
}

pub fn api_create_vu_stream(sink: StreamSink<Vec<f32>>) {
    let session_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    VU_THREAD_RUNNING.store(session_id, std::sync::atomic::Ordering::Relaxed);

    std::thread::spawn(move || loop {
        if VU_THREAD_RUNNING.load(std::sync::atomic::Ordering::Relaxed) != session_id {
            break;
        }
        let levels: Vec<f32> = GLOBAL_STATE
            .vu_levels
            .iter()
            .map(|v| f32::from_bits(v.load(std::sync::atomic::Ordering::Relaxed)))
            .collect();
        if sink.add(levels).is_err() {
            break; // Stop thread if port is closed
        }
        std::thread::sleep(std::time::Duration::from_millis(16));
    });
}

lazy_static::lazy_static! {
    static ref ENGINE_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    static ref ENGINE_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    static ref ENGINE_THREAD: std::sync::Mutex<Option<std::thread::JoinHandle<()>>> = std::sync::Mutex::new(None);
}

pub fn api_init_audio_system(device_name: Option<String>) -> Result<(), AtmosError> {
    let rx = GLOBAL_STATE.command_receiver.clone();
    
    api_stop_audio_engine();
    
    let gen = ENGINE_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    let (tx, rx_init) = std::sync::mpsc::channel();
    
    let prev_handle = {
        if let Ok(mut guard) = ENGINE_THREAD.lock() {
            guard.take()
        } else {
            None
        }
    };
    
    let handle = std::thread::spawn(move || {
        // 백그라운드 스레드 내에서 이전 스레드가 완전히 종료될 때까지 대기 (UI 프리징 방지)
        if let Some(h) = prev_handle {
            let _ = h.join();
        }

        std::thread::sleep(std::time::Duration::from_millis(100)); // safe margin
        
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
            unsafe {
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            }
        }
        
        let mut engine = crate::audio::engine::AudioEngine::new();
        {
            let (lock, _) = &**crate::audio::engine::ENGINE_INIT_SIGNAL;
            let mut fired = lock.lock().unwrap();
            *fired = false;
        }
        let device_name_clone = device_name.clone();
        let dummy_rx = rx.clone();
        match engine.start(device_name_clone, rx) {
            Ok(_) => {
                ENGINE_ACTIVE.store(true, std::sync::atomic::Ordering::SeqCst);
                
                // Wait for callback to actually fire using Condvar
                {
                    let (lock, cvar) = &**crate::audio::engine::ENGINE_INIT_SIGNAL;
                    let fired = lock.lock().unwrap();
                    if !*fired {
                        let _ = cvar.wait_timeout(fired, std::time::Duration::from_millis(2000)).unwrap();
                    }
                }
                
                let _ = tx.send(Ok(()));
                
                // Keep thread alive until next generation, and watch for DeviceNotAvailable
                loop {
                    if ENGINE_GENERATION.load(std::sync::atomic::Ordering::SeqCst) != gen {
                        break;
                    }
                    if let Some(err) = crate::core::state::GLOBAL_STATE.engine_error.read().unwrap_or_else(|e| e.into_inner()).as_ref() {
                        if err == "DeviceNotAvailable" {
                            println!("⚠️ [디버깅] 오디오 장치 유실 감지! 3초 후 자동 재연결 시도... (Dummy Callback 활성화)");
                            let dummy_rx_clone = dummy_rx.clone();
                            std::thread::spawn(move || {
                                // 30초(300 * 100ms) 동안 큐를 비워줌 (USB Hot-unplug Dummy Callback)
                                for _ in 0..300 {
                                    while let Ok(cmd) = dummy_rx_clone.try_recv() {
                                        if let crate::common::commands::AudioCommand::PlayTrack { instance_id, .. } = cmd {
                                            crate::core::state::GLOBAL_STATE.remove_playing_track(instance_id);
                                        }
                                    }
                                    std::thread::sleep(std::time::Duration::from_millis(100));
                                }
                            });
                            break;
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                
                drop(engine);
                ENGINE_ACTIVE.store(false, std::sync::atomic::Ordering::SeqCst);
                
                // If it wasn't a manual stop, auto restart
                if ENGINE_GENERATION.load(std::sync::atomic::Ordering::SeqCst) == gen {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    if ENGINE_GENERATION.load(std::sync::atomic::Ordering::SeqCst) == gen {
                        println!("🔄 [디버깅] 자동 재연결 수행!");
                        api_start_audio_engine(device_name.clone());
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(Err(e));
                
                // Auto reconnect on initial boot failure if it was DeviceNotAvailable
                // Wait, boot failures are returned immediately. So we don't block here.
            }
        }
    });
    
    if let Ok(mut guard) = ENGINE_THREAD.lock() {
        *guard = Some(handle);
    }
    
    rx_init.recv().unwrap_or_else(|_| Err("Failed to communicate with audio thread".to_string())).map_err(|e| AtmosError {
        message: format!("Failed to start audio engine: {}", e),
    })
}

pub fn api_start_audio_engine(device_name: Option<String>) {
    let _ = api_init_audio_system(device_name);
}

pub fn api_is_engine_ready() -> bool {
    if !ENGINE_ACTIVE.load(std::sync::atomic::Ordering::SeqCst) {
        return false;
    }
    let (lock, _) = &**crate::audio::engine::ENGINE_INIT_SIGNAL;
    if let Ok(fired) = lock.lock() {
        *fired
    } else {
        false
    }
}

pub fn api_stop_audio_engine() {
    let _ = ENGINE_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    println!("✅ [디버깅] 백엔드 오디오 엔진 명시적 종료 지시 완료. (비동기 종료 진행)");
}

pub fn api_open_asio_panel() {
    #[cfg(target_os = "windows")]
    {
        crate::core::state::GLOBAL_STATE.log("ASIO Control panel opening is not fully supported yet without custom asio-sys bindings.".to_string());
        // Custom ASIO control panel invocation would go here
    }
}

pub fn api_create_device_event_stream(sink: StreamSink<String>) {
    std::thread::spawn(move || {
        let mut last_err: Option<String> = None;
        loop {
            let current_err = GLOBAL_STATE.engine_error.read().unwrap_or_else(|e| e.into_inner()).clone();
            if current_err != last_err {
                if let Some(ref err) = current_err {
                    if (err == "DeviceNotAvailable" || err.contains("Disconnected")) && sink.add(err.clone()).is_err() {
                        break; // Stop thread if port is closed
                    }
                }
                last_err = current_err;
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    });
}

pub fn api_force_restart_engine(device_name: Option<String>) {
    println!("🔄 [디버깅] 백엔드 오디오 엔진 강제 재시작 요청됨.");
    api_stop_audio_engine();

    let _ = api_init_audio_system(device_name);
}

pub fn api_start_osc_listener(port: u16) {
    let listener = crate::osc::listener::OscListener::new();
    listener.start(port);
}

#[derive(Debug, Clone)]
pub struct EngineStateUpdate {
    pub active_room_id: Option<String>,
    pub ducking_active: bool,
    pub playing_track_ids: Vec<String>,
    pub engine_error: Option<String>,
    pub output_channel_count: u32,
}

pub fn api_create_engine_state_stream(sink: StreamSink<EngineStateUpdate>) {
    let playing_track_ids = {
        let guard = GLOBAL_STATE.playing_track_ids.read().unwrap_or_else(|e| e.into_inner());
        let mut unique_ids: Vec<String> = guard.values().cloned().collect();
        unique_ids.sort();
        unique_ids.dedup();
        unique_ids
    };

    let initial_state = EngineStateUpdate {
        active_room_id: GLOBAL_STATE.active_room_id.read().unwrap_or_else(|e| e.into_inner()).clone(),
        ducking_active: GLOBAL_STATE
            .is_ducking
            .load(std::sync::atomic::Ordering::Relaxed),
        playing_track_ids,
        engine_error: GLOBAL_STATE.engine_error.read().unwrap_or_else(|e| e.into_inner()).clone(),
        output_channel_count: GLOBAL_STATE.active_device_channels.load(std::sync::atomic::Ordering::Relaxed),
    };
    let _ = sink.add(initial_state);
    *GLOBAL_STATE.state_sink.write().unwrap_or_else(|e| e.into_inner()) = Some(sink);
}

pub fn api_preload_all_sounds(config: AppConfig) -> Result<(), AtmosError> {
    let mut needed_files = std::collections::HashSet::new();
    for room in &config.rooms {
        for track in &room.tracks {
            if !track.is_loop && !track.is_streaming {
                needed_files.insert(track.file_path.clone());
            }
        }
    }

    let mut missing_files = Vec::new();
    {
        let cache = GLOBAL_STATE.sound_cache.read().unwrap_or_else(|e| e.into_inner());
        for file in &needed_files {
            if !cache.contains_key(file) {
                missing_files.push(file.clone());
            }
        }
    }

    let mut newly_loaded = Vec::new();
    let mut errors = Vec::new();
    for file in missing_files {
        let path = std::path::Path::new(&file);
        
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() > 500 * 1024 * 1024 {
                let err_msg = format!("파일 용량이 너무 큽니다 (500MB 초과). BGM(Loop)으로 설정하거나 용량을 줄이세요: {}", file);
                GLOBAL_STATE.log(err_msg.clone());
                errors.push(err_msg);
                continue;
            }
        }
        
        // Load into RAM regardless of size (SFX only, loop=false)
        match crate::audio::player::SoundData::load_from_file(path) {
            Ok(data) => {
                GLOBAL_STATE.log(format!("Loaded sound file: {}", file));
                newly_loaded.push((file, std::sync::Arc::new(data)));
            }
            Err(e) => {
                let err_msg = format!("Failed to load sound file {}: {}", file, e);
                GLOBAL_STATE.log(err_msg.clone());
                errors.push(err_msg);

                // Negative cache to prevent disk spam on every UI tick
                let empty_data = crate::audio::player::SoundData {
                    sample_rate: 48000,
                    channels: 2,
                    samples: vec![],
                };
                newly_loaded.push((file, std::sync::Arc::new(empty_data)));
            }
        }
    }

    {
        let mut cache = GLOBAL_STATE.sound_cache.write().unwrap_or_else(|e| e.into_inner());
        cache.retain(|path, _| needed_files.contains(path));
        for (path, data) in newly_loaded {
            cache.insert(path, data);
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
                    GLOBAL_STATE.enabled_channels[real_ch]
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
                if real_ch + 1 < GLOBAL_STATE.enabled_channels.len() {
                    GLOBAL_STATE.enabled_channels[real_ch + 1]
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
        for (&ch, setting) in &config.stereo_configs {
            if setting.enabled && ch > 0 {
                let real_ch = (ch - 1) as usize;
                if real_ch < GLOBAL_STATE.enabled_channels.len() {
                    GLOBAL_STATE.enabled_channels[real_ch]
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
                if real_ch + 1 < GLOBAL_STATE.enabled_channels.len() {
                    GLOBAL_STATE.enabled_channels[real_ch + 1]
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
    }

    let mut global_config = GLOBAL_STATE.config.write().unwrap_or_else(|e| e.into_inner());
    *global_config = Some(config.clone());
    drop(global_config);

    // Check if active_room_id exists in the new config
    let active_room_id = {
        let guard = GLOBAL_STATE.active_room_id.read().unwrap_or_else(|e| e.into_inner());
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
        return Err(AtmosError {
            message: errors.join(" | "),
        });
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
            if let Some(config) = crate::core::state::GLOBAL_STATE
                .config
                .read()
                .unwrap()
                .as_ref()
            {
                if let Some(ref saved_name) = config.device_name {
                    if saved_name.starts_with("[ASIO]") {
                        skip_asio_scan = true;
                        active_asio_device = Some(saved_name.clone());
                    }
                }
            }
        }

        let target_prefix = if skip_asio_scan {
            Some("[WASAPI]")
        } else {
            None
        };
        let hosts = crate::audio::engine::get_hosts(target_prefix).map_err(|e| AtmosError { message: e })?;

        let mut device_info_list = Vec::new();

        if skip_asio_scan {
            if let Some(saved_name) = active_asio_device {
                let active_ch = crate::core::state::GLOBAL_STATE
                    .active_device_channels
                    .load(std::sync::atomic::Ordering::SeqCst);
                let max_channels = if active_ch > 0 { active_ch } else { 2 };
                let actual_name = saved_name.replace("[ASIO] ", "").trim().to_string();
                #[cfg(target_os = "macos")]
                let channel_names =
                    crate::audio::channel_names::get_channel_names_mac(&actual_name, max_channels);
                #[cfg(target_os = "windows")]
                let channel_names =
                    crate::audio::channel_names::get_channel_names_win(&actual_name, max_channels);
                #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                let channel_names =
                    crate::audio::channel_names::get_channel_names_fallback(max_channels);

                device_info_list.push(OutputDeviceInfo {
                    name: saved_name,
                    max_channels,
                    channel_names,
                });
            }
        }

        for host in hosts {
            let prefix = format!("[{}] ", host.id().name());
            let is_asio_host = host.id().name() == "ASIO";

            if is_asio_host && skip_asio_scan {
                continue; // Do not call host.output_devices() because querying ASIO devices breaks the COM lock!
            }

            let devices = match host.output_devices() {
                Ok(d) => d,
                Err(e) => {
                    eprintln!(
                        "Failed to get output devices for host {}: {:?}",
                        host.id().name(),
                        e
                    );
                    continue;
                }
            };

            for device in devices {
                if let Ok(name_str) = device.name() {
                    let actual_name = name_str.replace('\0', "").trim().to_string();
                    let name = format!("{}{}", prefix, actual_name);
                    let mut max_channels = 2; // Default fallback

                    // Avoid querying configs if it's ASIO (though skip_asio_scan handles active ASIO lock already)
                    if is_asio_host && is_engine_active {
                        let mut is_the_active_device = false;
                        if let Some(config) = crate::core::state::GLOBAL_STATE
                            .config
                            .read()
                            .unwrap()
                            .as_ref()
                        {
                            if let Some(ref saved_name) = config.device_name {
                                if saved_name.trim() == name.trim() {
                                    is_the_active_device = true;
                                }
                            }
                        }

                        if is_the_active_device {
                            let active_ch = crate::core::state::GLOBAL_STATE
                                .active_device_channels
                                .load(std::sync::atomic::Ordering::SeqCst);
                            max_channels = if active_ch > 0 { active_ch } else { 2 };
                        } else {
                            max_channels = 2;
                        }
                    } else {
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
                    }

                    #[cfg(target_os = "macos")]
                    let channel_names = crate::audio::channel_names::get_channel_names_mac(
                        &actual_name,
                        max_channels,
                    );
                    #[cfg(target_os = "windows")]
                    let channel_names = crate::audio::channel_names::get_channel_names_win(
                        &actual_name,
                        max_channels,
                    );
                    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                    let channel_names =
                        crate::audio::channel_names::get_channel_names_fallback(max_channels);

                    device_info_list.push(OutputDeviceInfo {
                        name,
                        max_channels,
                        channel_names,
                    });
                }
            }
        }

        Ok(device_info_list)
    })
    .join()
    .unwrap_or_else(|_| {
        Err(AtmosError {
            message: "Thread panicked during device lookup".to_string(),
        })
    })
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
            if let Some(config) = crate::core::state::GLOBAL_STATE
                .config
                .read()
                .unwrap()
                .as_ref()
            {
                if let Some(ref saved_name) = config.device_name {
                    if saved_name.starts_with("[ASIO]") {
                        active_asio_device = Some(saved_name.clone());
                    }
                }
            }
        }

        let device = if let Some(ref name) = device_name {
            if is_engine_active && name.starts_with("[ASIO]") {
                if let Some(ref active_name) = active_asio_device {
                    if name.trim() == active_name.trim() {
                        let active_ch = crate::core::state::GLOBAL_STATE
                            .active_device_channels
                            .load(std::sync::atomic::Ordering::SeqCst);
                        return Ok(if active_ch > 0 { active_ch } else { 2 });
                    }
                }
                return Ok(2); // Any inactive ASIO device defaults to 2 (stereo) while engine is running
            }

            let target_prefix = if name.starts_with("[ASIO]") {
                Some("[ASIO]")
            } else if name.starts_with("[WASAPI]") {
                Some("[WASAPI]")
            } else if name.starts_with("[CoreAudio]") {
                Some("[CoreAudio]")
            } else {
                None
            };

            let hosts = crate::audio::engine::get_hosts(target_prefix)
                .map_err(|e| AtmosError { message: e })?;

            let mut found_device = None;
            let target_name = name
                .replace("[ASIO] ", "")
                .replace("[WASAPI] ", "")
                .replace("[CoreAudio] ", "");
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
            found_device.ok_or_else(|| AtmosError {
                message: format!("Device not found: {}", name),
            })?
        } else {
            cpal::default_host()
                .default_output_device()
                .ok_or_else(|| AtmosError {
                    message: "No default output device".to_string(),
                })?
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
    })
    .join()
    .unwrap_or_else(|_| {
        Err(AtmosError {
            message: "Thread panicked during channel count".to_string(),
        })
    })
}

pub fn api_get_device_channel_names(
    device_name: Option<String>,
) -> Result<Vec<String>, AtmosError> {
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
    let channel_names =
        crate::audio::channel_names::get_channel_names_mac(&actual_name, max_channels);
    #[cfg(target_os = "windows")]
    let channel_names =
        crate::audio::channel_names::get_channel_names_win(&actual_name, max_channels);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let channel_names = crate::audio::channel_names::get_channel_names_fallback(max_channels);

    Ok(channel_names)
}

pub fn api_export_logs(destination_dir: String) -> Result<(), AtmosError> {
    let mut dir = std::env::temp_dir();
    dir.push("atmos_mixer_pro_logs");
    dir.push("atmos_mixer_pro.log");
    if dir.exists() {
        let dest_path = std::path::Path::new(&destination_dir).join("atmos_mixer_pro.log");
        std::fs::copy(&dir, &dest_path).map_err(|e| AtmosError {
            message: format!("Failed to copy log file: {}", e),
        })?;
        Ok(())
    } else {
        Err(AtmosError {
            message: "Log file does not exist".to_string(),
        })
    }
}

pub fn api_play_all_loop_tracks() -> Result<(), AtmosError> {
    let config = {
        let guard = GLOBAL_STATE.config.read().unwrap_or_else(|e| e.into_inner());
        guard.as_ref().cloned()
    };
    
    if let Some(config) = config {
        for room in config.rooms {
            for track in room.tracks {
                if track.is_loop {
                    let _ = api_play_track(room.id.clone(), track.id.clone());
                }
            }
        }
    }
    Ok(())
}

pub fn api_load_preset(config: AppConfig) -> Result<(), AtmosError> {
    api_stop_all()?;
    
    GLOBAL_STATE.is_exhibition_mode.store(config.is_exhibition_mode, std::sync::atomic::Ordering::Relaxed);
    
    // This will sync GLOBAL_STATE config, enabled_channels, and manage the cache
    api_preload_all_sounds(config)?;
    Ok(())
}

pub fn api_trigger_test_error(message: String) -> Result<(), AtmosError> {
    *GLOBAL_STATE.engine_error.write().unwrap_or_else(|e| e.into_inner()) = Some(message);
    GLOBAL_STATE.broadcast_state();
    Ok(())
}

pub fn api_get_audio_file_channels(file_path: String) -> u32 {
    if let Ok(cache) = crate::core::state::GLOBAL_STATE.sound_cache.read() {
        if let Some(data) = cache.get(&file_path) {
            return data.channels as u32;
        }
    }
    let path = std::path::Path::new(&file_path);
    crate::audio::player::SoundData::probe_channels(path)
}

use crate::common::config::EqBand;

pub struct ChannelTuningParams {
    pub channel: u32,
    pub delay_ms: f32,
    pub eq_bands: Vec<EqBand>,
}

pub fn api_apply_all_channel_tunings(tunings: Vec<ChannelTuningParams>) -> Result<(), AtmosError> {
    let cmd_tunings = tunings.iter().map(|t| (t.channel as usize, t.delay_ms, t.eq_bands.clone())).collect();
    GLOBAL_STATE
        .command_sender
        .try_send(AudioCommand::ApplyAllChannelTunings {
            tunings: cmd_tunings,
        })
        .map_err(|e| AtmosError {
            message: format!("Failed to apply all channel tunings: {}", e),
        })?;

    if let Ok(mut config_guard) = GLOBAL_STATE.config.write() {
        if let Some(config) = config_guard.as_mut() {
            for t in tunings {
                let ch_key = t.channel + 1;
                if let Some(setting) = config.mono_configs.get_mut(&ch_key) {
                    setting.delay_ms = t.delay_ms;
                    setting.eq_bands = t.eq_bands.clone();
                } else if let Some(setting) = config.stereo_configs.get_mut(&ch_key) {
                    setting.delay_ms = t.delay_ms;
                    setting.eq_bands = t.eq_bands.clone();
                } else if let Some(setting) = config.multi_configs.get_mut(&ch_key) {
                    setting.delay_ms = t.delay_ms;
                    setting.eq_bands = t.eq_bands.clone();
                }
            }
        }
    }

    Ok(())
}

pub fn api_apply_channel_tuning(channel: u32, delay_ms: f32, eq_bands: Vec<EqBand>) -> Result<(), AtmosError> {
    println!("🔥 [디버깅] api_apply_channel_tuning 호출됨. 채널: {}, 딜레이: {}ms", channel, delay_ms);
    GLOBAL_STATE
        .command_sender
        .try_send(AudioCommand::ApplyChannelTuning {
            channel: channel as usize,
            delay_ms,
            eq_bands: eq_bands.clone(), // Send the EqBands directly now
        })
        .map_err(|e| AtmosError {
            message: format!("Failed to apply channel tuning: {}", e),
        })?;

    // 인메모리 Config 상태 업데이트 (믹서 재시작 시 복구용)
    if let Ok(mut config_guard) = GLOBAL_STATE.config.write() {
        if let Some(config) = config_guard.as_mut() {
            let ch_key = channel + 1;
            if let Some(setting) = config.mono_configs.get_mut(&ch_key) {
                setting.delay_ms = delay_ms;
                setting.eq_bands = eq_bands.clone();
            } else if let Some(setting) = config.stereo_configs.get_mut(&ch_key) {
                setting.delay_ms = delay_ms;
                setting.eq_bands = eq_bands.clone();
            } else if let Some(setting) = config.multi_configs.get_mut(&ch_key) {
                setting.delay_ms = delay_ms;
                setting.eq_bands = eq_bands.clone();
            }
        }
    }

    Ok(())
}


pub fn api_apply_global_tuning(master_headroom_db: f32, peak_limiter_enabled: bool) -> Result<(), AtmosError> {
    GLOBAL_STATE
        .command_sender
        .try_send(AudioCommand::ApplyGlobalTuning {
            master_headroom_db,
            peak_limiter_enabled,
        })
        .map_err(|e| AtmosError {
            message: format!("Failed to apply global tuning: {}", e),
        })?;

    if let Ok(mut config_guard) = GLOBAL_STATE.config.write() {
        if let Some(config) = config_guard.as_mut() {
            config.master_headroom_db = master_headroom_db;
            config.peak_limiter_enabled = peak_limiter_enabled;
        }
    }
    Ok(())
}

#[derive(serde::Deserialize)]
struct SpatialConfigPayload {
    channel_positions: Vec<Option<crate::common::config::Point3D>>,
    room_zones: Vec<crate::common::config::RoomZone>,
    trajectory: Option<crate::common::config::Trajectory>,
}

pub fn api_update_spatial_config_json(json_payload: String) -> Result<(), AtmosError> {
    let payload: SpatialConfigPayload = serde_json::from_str(&json_payload).map_err(|e| AtmosError {
        message: format!("Failed to parse spatial config JSON: {}", e),
    })?;

    GLOBAL_STATE
        .command_sender
        .try_send(AudioCommand::UpdateSpatialConfig {
            channel_positions: payload.channel_positions,
            room_zones: payload.room_zones,
            trajectory: payload.trajectory,
        })
        .map_err(|e| AtmosError {
            message: format!("Failed to send UpdateSpatialConfig: {}", e),
        })?;

    Ok(())
}
