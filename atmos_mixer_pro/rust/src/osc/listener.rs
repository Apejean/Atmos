use crate::osc::debouncer::OscDebouncer;
use crate::osc::router::{get_osc_action, OscAction};
use rosc::OscPacket;
use std::net::UdpSocket;
use std::sync::Arc;
use std::thread;

use crate::common::utils::hash_id;
use lazy_static::lazy_static;
use std::sync::atomic::{AtomicBool, Ordering};

lazy_static! {
    pub static ref OSC_RUNNING_FLAG: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
}

pub struct OscListener {
    debouncer: Arc<OscDebouncer>,
}

impl Default for OscListener {
    fn default() -> Self {
        Self::new()
    }
}

impl OscListener {
    pub fn new() -> Self {
        Self {
            debouncer: Arc::new(OscDebouncer::new()),
        }
    }

    pub fn start(&self, port: u16) {
        OSC_RUNNING_FLAG.store(false, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_millis(600)); // wait for old to die
        OSC_RUNNING_FLAG.store(true, Ordering::Relaxed);

        let ports_to_listen = if port != 53000 {
            vec![port, 53000] // Always include QLab default port
        } else {
            vec![port]
        };

        for p in ports_to_listen {
            if p == 0 { continue; }
            let debouncer = self.debouncer.clone();
            let running_flag = OSC_RUNNING_FLAG.clone();
            thread::spawn(move || {
                let addr = format!("0.0.0.0:{}", p);
                let socket = match UdpSocket::bind(&addr) {
                    Ok(s) => s,
                    Err(e) => {
                        let err_msg = format!("Failed to bind OSC port {}: {}", p, e);
                        println!("{}", err_msg);
                        crate::core::state::GLOBAL_STATE.log(err_msg);
                        return;
                    }
                };
                if let Err(e) = socket.set_read_timeout(Some(std::time::Duration::from_millis(500))) {
                    println!("Warning: Failed to set read timeout on OSC socket: {}", e);
                }
                crate::core::state::GLOBAL_STATE.log(format!("OSC Listener started on {}", addr));

                let mut buf = [0u8; rosc::decoder::MTU];
                loop {
                    if !running_flag.load(Ordering::Relaxed) {
                        crate::core::state::GLOBAL_STATE.log("OSC Listener stopping...".to_string());
                        break;
                    }
                    match socket.recv_from(&mut buf) {
                        Ok((size, _addr)) => {
                            match rosc::decoder::decode_udp(&buf[..size]) {
                                Ok((_, packet)) => {
                                    let addr = match &packet {
                                        OscPacket::Message(m) => Some(m.addr.as_str()),
                                        OscPacket::Bundle(_) => Some("#bundle"),
                                    };
                                    crate::osc::metrics::GLOBAL_OSC_METRICS.record_packet(size, true, addr);
                                    handle_packet(packet, &debouncer);
                                }
                                Err(_) => {
                                    crate::osc::metrics::GLOBAL_OSC_METRICS.record_packet(size, false, None);
                                }
                            }
                        }
                        Err(_e) => {
                            // Timeout or other error
                            continue;
                        }
                    }
                }
            });
        }
    }
}

fn check_gating(room_id: &str, is_exhibition: bool) -> bool {
    if is_exhibition { return true; }
    let active = crate::core::state::GLOBAL_STATE.active_room_id.read().unwrap_or_else(|e| e.into_inner());
    if let Some(ref active_id) = *active {
        return active_id == room_id;
    }
    true
}

fn handle_packet(packet: OscPacket, debouncer: &OscDebouncer) {
    match packet {
        OscPacket::Message(msg) => {
            let config_version = crate::core::state::GLOBAL_STATE.config_version.load(Ordering::Relaxed);
            let config_guard = crate::core::state::GLOBAL_STATE.config.read().unwrap_or_else(|e| e.into_inner());
            let config = match *config_guard {
                Some(ref c) => c.clone(),
                None => return,
            };
            drop(config_guard); // Free lock early!

            // 1. Dynamic Paths (Tracking)
            if let Some(ref tracking_addr) = config.tracking_osc_address {
                if !tracking_addr.is_empty() && msg.addr == *tracking_addr {
                    if msg.args.len() >= 3 {
                        let mut coords = [0.0; 3];
                        for (i, arg) in msg.args.iter().take(3).enumerate() {
                            if let rosc::OscType::Float(f) = arg {
                                coords[i] = *f;
                            }
                        }
                        let _ = crate::core::state::GLOBAL_STATE.command_sender.send(
                            crate::common::commands::AudioCommand::UpdateTrajectoryPosition {
                                position: crate::common::config::Point3D {
                                    x: coords[0],
                                    y: coords[1],
                                    z: coords[2],
                                    ..Default::default()
                                },
                            }
                        );
                    }
                    return;
                }
            }

            // Hardcoded audience pos
            if msg.addr == "/audience/pos" {
                if msg.args.len() >= 3 {
                    let mut x = 0.0;
                    let mut y = 0.0;
                    let mut z = 0.0;
                    if let rosc::OscType::Float(f) = msg.args[0] { x = f; }
                    if let rosc::OscType::Float(f) = msg.args[1] { y = f; }
                    if let rosc::OscType::Float(f) = msg.args[2] { z = f; }
                    let _ = crate::core::state::GLOBAL_STATE.command_sender.send(
                        crate::common::commands::AudioCommand::UpdateTrajectoryPosition {
                            position: crate::common::config::Point3D { x, y, z, ..Default::default() },
                        }
                    );
                }
                return;
            }
            
            // Dynamic path: /atmos/track/{ch}/volume
            if msg.addr.starts_with("/atmos/track/") && msg.addr.ends_with("/volume") {
                let parts: Vec<&str> = msg.addr.split('/').collect();
                if parts.len() == 5 { // "", "atmos", "track", "{ch}", "volume"
                    if let Ok(ch) = parts[3].parse::<usize>() {
                        if let Some(arg) = msg.args.get(0) {
                            let vol = match arg {
                                rosc::OscType::Float(f) => *f,
                                rosc::OscType::Double(d) => *d as f32,
                                rosc::OscType::Int(i) => *i as f32,
                                _ => 0.0,
                            };
                            let _ = crate::core::state::GLOBAL_STATE.command_sender.send(
                                crate::common::commands::AudioCommand::SetTrackVolume {
                                    room_id: 0, // Not room specific
                                    track_id: ch as u32, // Just a placeholder or needs actual mapping
                                    volume: vol.clamp(0.0, 1.0),
                                }
                            );
                            return;
                        }
                    }
                }
            }

            // For static triggers, check trigger condition and debounce
            let is_trigger = msg.args.iter().any(|arg| match arg {
                rosc::OscType::Float(f) => *f > 0.0,
                rosc::OscType::Int(i) => *i > 0,
                _ => true,
            });
            
            if !is_trigger || !debouncer.should_process(&msg.addr) {
                return;
            }

            // 2. High-performance Router (HashMap cached)
            if let Some(action) = get_osc_action(&msg.addr, &config, config_version) {
                crate::core::state::GLOBAL_STATE.log(format!("Valid OSC Action: {}", msg.addr));
                match action {
                    OscAction::SystemReset => {
                        let _ = crate::api::simple::api_stop_all();
                        crate::core::state::GLOBAL_STATE.log("OSC Triggered: System Reset".to_string());
                    },
                    OscAction::ThemeStart(first_room, track_ids) => {
                        let _ = crate::api::simple::api_stop_all();
                        if !first_room.is_empty() {
                            let _ = crate::api::simple::api_set_active_room(Some(first_room.clone()));
                            for track_id in track_ids {
                                let _ = crate::api::simple::api_play_track(first_room.clone(), track_id);
                            }
                        }
                    },
                    OscAction::ClearRoom(room_id) => {
                        if !check_gating(&room_id, config.is_exhibition_mode) { return; }
                        
                        crate::core::state::GLOBAL_STATE.clear_playing_tracks();
                        let _ = crate::core::state::GLOBAL_STATE.command_sender.send(
                            crate::common::commands::AudioCommand::ClearRoom {
                                room_id: hash_id(&room_id),
                            },
                        );

                        // Auto promote next room
                        let mut next_room_id = None;
                        let mut found_current = false;
                        for r in &config.rooms {
                            if found_current {
                                next_room_id = Some(r.id.clone());
                                break;
                            }
                            if r.id == room_id { found_current = true; }
                        }
                        
                        if let Some(next_id) = next_room_id {
                            crate::core::state::GLOBAL_STATE.set_active_room(Some(next_id.clone()));
                            crate::core::state::GLOBAL_STATE.log(format!("Interlock: Auto-promoted room {} to active", next_id));
                            
                            // Auto-play bgm
                            if let Some(next_r) = config.rooms.iter().find(|r| r.id == next_id) {
                                for next_t in next_r.tracks.iter().filter(|t| t.is_loop) {
                                    let data_opt = {
                                        let cache_guard = crate::core::state::GLOBAL_STATE.sound_cache.read().unwrap_or_else(|e| e.into_inner());
                                        cache_guard.get(&next_t.file_path).cloned()
                                    };
                                    if let Some(data) = data_opt {
                                        let playing = crate::core::state::GLOBAL_STATE.playing_track_ids.read().unwrap_or_else(|e| e.into_inner());
                                        let is_playing = playing.values().any(|id| id == &next_t.id);
                                        drop(playing);

                                        if !is_playing {
                                            let instance_id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64;
                                            crate::core::state::GLOBAL_STATE.add_playing_track(instance_id, next_t.id.clone());
                                            let _ = crate::core::state::GLOBAL_STATE.command_sender.send(crate::common::commands::AudioCommand::PlayTrack {
                                                instance_id,
                                                room_id: hash_id(&next_id),
                                                track_id: hash_id(&next_t.id),
                                                track_id_str: next_t.id.clone(),
                                                data: Some(data.clone()),
                                                stream_receiver: None,
                                                stream_sample_rate: data.sample_rate,
                                                stream_channels: data.channels,
                                                is_loop: next_t.is_loop,
                                                volume: next_t.volume,
                                                output_channel: next_t.output_channel as usize,
                                                output_stereo: next_t.output_stereo,
                                                current_position: None,
                                            });
                                        }
                                    }
                                }
                            }
                        } else {
                            crate::core::state::GLOBAL_STATE.set_active_room(None);
                        }
                    },
                    OscAction::SetMasterVolume(room_id) => {
                        if let Some(arg) = msg.args.get(0) {
                            let vol = match arg {
                                rosc::OscType::Float(f) => *f,
                                rosc::OscType::Double(d) => *d as f32,
                                rosc::OscType::Int(i) => *i as f32,
                                _ => 0.0,
                            };
                            let _ = crate::core::state::GLOBAL_STATE.command_sender.send(
                                crate::common::commands::AudioCommand::SetMasterVolume {
                                    room_id: hash_id(&room_id),
                                    volume: vol.clamp(0.0, 1.0),
                                },
                            );
                        }
                    },
                    OscAction::PlayTrack(room_id, track_id) => {
                        if !check_gating(&room_id, config.is_exhibition_mode) { return; }
                        
                        if let Some(r) = config.rooms.iter().find(|x| x.id == room_id) {
                            if let Some(t) = r.tracks.iter().find(|x| x.id == track_id) {
                                let data_opt = {
                                    let cache = crate::core::state::GLOBAL_STATE.sound_cache.read().unwrap_or_else(|e| e.into_inner());
                                    cache.get(&t.file_path).cloned()
                                };
                                if let Some(data) = data_opt {
                                    let instance_id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64;
                                    crate::core::state::GLOBAL_STATE.add_playing_track(instance_id, t.id.clone());
                                    let _ = crate::core::state::GLOBAL_STATE.command_sender.send(crate::common::commands::AudioCommand::PlayTrack {
                                        instance_id,
                                        room_id: hash_id(&room_id),
                                        track_id: hash_id(&track_id),
                                        track_id_str: t.id.clone(),
                                        data: Some(data.clone()),
                                        stream_receiver: None,
                                        stream_sample_rate: data.sample_rate,
                                        stream_channels: data.channels,
                                        is_loop: t.is_loop,
                                        volume: t.volume,
                                        output_channel: t.output_channel as usize,
                                        output_stereo: t.output_stereo,
                                        current_position: None,
                                    });
                                }
                            }
                        }
                    },
                    OscAction::StopTrack(room_id, track_id) => {
                        if !check_gating(&room_id, config.is_exhibition_mode) { return; }
                        let _ = crate::core::state::GLOBAL_STATE.command_sender.send(
                            crate::common::commands::AudioCommand::StopTrack {
                                room_id: hash_id(&room_id),
                                track_id: hash_id(&track_id),
                            },
                        );
                    }
                }
            } else {
                crate::core::state::GLOBAL_STATE.log(format!("No matching track found for OSC address: {}", msg.addr));
            }
        }
        OscPacket::Bundle(bundle) => {
            for packet in bundle.content {
                handle_packet(packet, debouncer);
            }
        }
    }
}
