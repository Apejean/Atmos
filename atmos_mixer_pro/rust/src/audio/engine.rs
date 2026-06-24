use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig, OutputCallbackInfo, SampleFormat};
use crossbeam_channel::Receiver;
use crate::audio::mixer::AudioMixer;
use crate::common::commands::AudioCommand;

pub struct AudioEngine {
    stream: Option<Stream>,
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
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

impl AudioEngine {
    pub fn new() -> Self {
        Self {
            stream: None,
        }
    }

    pub fn start(&mut self, device_name: Option<String>, cmd_receiver: Receiver<AudioCommand>) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            }
        }

        let target_prefix = device_name.as_ref().and_then(|n| {
            if n.starts_with("[ASIO]") { Some("[ASIO]") } 
            else if n.starts_with("[WASAPI]") { Some("[WASAPI]") } 
            else { None }
        });
        
        let hosts = get_hosts(target_prefix)?;
        
        let device = if let Some(ref name) = device_name {
            let mut found_device = None;
            for host in &hosts {
                let prefix = format!("[{}] ", host.id().name());
                if name.starts_with(&prefix) {
                    let actual_name = name.strip_prefix(&prefix).unwrap_or(name).trim_matches(char::from(0)).trim();
                    if let Ok(devices) = host.output_devices() {
                        for d in devices {
                            if let Ok(d_name) = d.name() {
                                if d_name.trim_matches(char::from(0)).trim() == actual_name {
                                    found_device = Some(d);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            if let Some(d) = found_device {
                d
            } else {
                let error_msg = format!("Requested device '{}' not found. Available devices were not matched.", name);
                eprintln!("{}", error_msg);
                return Err(error_msg);
            }
        } else {
            cpal::default_host().default_output_device().ok_or("No default output device".to_string())?
        };

        println!("Using output device: {}", device.name().unwrap_or_default());

        let default_config_result = device.default_output_config().ok();
        let mut best_config = default_config_result.clone();
        let mut max_ch = best_config.as_ref().map(|c| c.channels()).unwrap_or(0);
        let default_sample_rate = best_config.as_ref().map(|c| c.sample_rate()).unwrap_or(cpal::SampleRate(48000));

        if let Ok(supported_configs) = device.supported_output_configs() {
            for c in supported_configs {
                if c.channels() > max_ch {
                    max_ch = c.channels();
                    if c.min_sample_rate() <= default_sample_rate && c.max_sample_rate() >= default_sample_rate {
                        best_config = Some(c.with_sample_rate(default_sample_rate));
                    } else {
                        best_config = Some(c.with_max_sample_rate());
                    }
                }
            }
        }

        let supported_config = best_config.expect("No output configs found");
        let sample_format = supported_config.sample_format();
        let config: StreamConfig = supported_config.into();

        println!("Stream config: {:?}", config);

        let (gc_tx, gc_rx) = crossbeam_channel::bounded::<crate::audio::player::SoundInstance>(256);
        std::thread::spawn(move || {
            while let Ok(dropped) = gc_rx.recv() {
                // Instance is dropped here in a background thread, preventing GC in audio thread.
                crate::core::state::GLOBAL_STATE.remove_playing_track(dropped.instance_id);
            }
        });

        let mut mixer = AudioMixer::new(config.sample_rate.0, gc_tx);

        let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

        let stream = match sample_format {
            SampleFormat::F32 => {
                device.build_output_stream(
                    &config,
                    move |data: &mut [f32], _: &OutputCallbackInfo| {
                        Self::process_commands(&mut mixer, &cmd_receiver);
                        mixer.process(data, config.channels as usize);
                    },
                    err_fn,
                    None
                )
            },
            SampleFormat::I16 => {
                let mut temp_buf: Vec<f32> = Vec::new();
                device.build_output_stream(
                    &config,
                    move |data: &mut [i16], _: &OutputCallbackInfo| {
                        Self::process_commands(&mut mixer, &cmd_receiver);
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
                    None
                )
            },
            SampleFormat::I32 => {
                let mut temp_buf: Vec<f32> = Vec::new();
                device.build_output_stream(
                    &config,
                    move |data: &mut [i32], _: &OutputCallbackInfo| {
                        Self::process_commands(&mut mixer, &cmd_receiver);
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
                    None
                )
            },
            SampleFormat::U16 => {
                let mut temp_buf: Vec<f32> = Vec::new();
                device.build_output_stream(
                    &config,
                    move |data: &mut [u16], _: &OutputCallbackInfo| {
                        Self::process_commands(&mut mixer, &cmd_receiver);
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
                    None
                )
            },
            _ => return Err("Unsupported format".to_string()),
        }.map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;
        self.stream = Some(stream);
        Ok(())
    }

    fn process_commands(mixer: &mut AudioMixer, rx: &Receiver<AudioCommand>) {
        // Lock-free pop from command queue
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                AudioCommand::PlayTrack { instance_id, room_id, track_id, track_id_str, data, stream_receiver, stream_sample_rate, stream_channels, is_loop, volume, output_channel, output_stereo } => {
                    let mut instance = crate::audio::player::SoundInstance::new(
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
                    );
                    instance.volume = volume;
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
                AudioCommand::ClearRoom { room_id } => {
                    for inst in mixer.instances.iter_mut().flatten() {
                        if inst.room_id == room_id {
                            inst.is_stopping = true;
                        }
                    }
                }
                AudioCommand::SetTrackVolume { room_id, track_id, volume } => {
                    for inst in mixer.instances.iter_mut().flatten() {
                        if inst.room_id == room_id && inst.id == track_id {
                            inst.volume = volume;
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
