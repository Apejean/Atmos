use rust_lib_atmos_mixer_pro::api::simple::*;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use std::thread;
use std::time::Duration;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref TEST_LOCK: Mutex<()> = Mutex::new(());
}

#[test]
fn test_room_clear_spam_no_duplicate() {
    let _guard = TEST_LOCK.lock().unwrap();
    println!("Starting room clear spam stress test...");
    
    // drain any pending commands
    let rx = GLOBAL_STATE.command_receiver.clone();
    while let Ok(_) = rx.try_recv() {}

    // Clear global state first
    api_stop_all().unwrap();
    
    // Simulate OSC listener loop processing 10 "clear room" messages back-to-back very quickly
    for _ in 0..10 {
        let next_track_id = "next_bgm_track".to_string();
        
        let playing = GLOBAL_STATE.playing_track_ids.read().unwrap();
        let is_playing = playing.values().any(|id| id == &next_track_id);
        drop(playing);
        
        if !is_playing {
            let instance_id = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
            GLOBAL_STATE.add_playing_track(instance_id, next_track_id.clone());
            let _ = GLOBAL_STATE.command_sender.try_send(AudioCommand::PlayTrack {
                instance_id,
                room_id: rust_lib_atmos_mixer_pro::common::utils::hash_id("next_room"),
                track_id: rust_lib_atmos_mixer_pro::common::utils::hash_id(&next_track_id),
                track_id_str: next_track_id,
                data: None,
                stream_receiver: None,
                stream_sample_rate: 44100,
                stream_channels: 2,
                is_loop: true,
                volume: 1.0,
                output_channel: 0,
                output_stereo: true,
            });
        }
    }
    
    let mut count = 0;
    while let Ok(cmd) = rx.try_recv() {
        if let AudioCommand::PlayTrack { track_id_str, .. } = cmd {
            if track_id_str == "next_bgm_track" {
                count += 1;
            }
        }
    }
    
    println!("BGM Play Track count after 10 clear room spams: {}", count);
    assert_eq!(count, 1, "Duplicate BGM playback detected! Expected 1, got {}", count);
    println!("Room clear spam stress test PASSED.");
}

#[test]
fn test_system_reset_theme_start_glitch() {
    let _guard = TEST_LOCK.lock().unwrap();
    println!("Starting system reset vs theme start glitch stress test...");
    
    // drain any pending commands
    let rx = GLOBAL_STATE.command_receiver.clone();
    while let Ok(_) = rx.try_recv() {}

    let num_iterations = 1000;
    
    let t1 = thread::spawn(move || {
        for i in 0..num_iterations {
            let _ = api_set_active_room(Some("room_1".to_string()));
            let instance_id = i as u64;
            GLOBAL_STATE.add_playing_track(instance_id, "theme_bgm".to_string());
            let _ = GLOBAL_STATE.command_sender.try_send(AudioCommand::PlayTrack {
                instance_id,
                room_id: rust_lib_atmos_mixer_pro::common::utils::hash_id("room_1"),
                track_id: rust_lib_atmos_mixer_pro::common::utils::hash_id("theme_bgm"),
                track_id_str: "theme_bgm".to_string(),
                data: None,
                stream_receiver: None,
                stream_sample_rate: 44100,
                stream_channels: 2,
                is_loop: true,
                volume: 1.0,
                output_channel: 0,
                output_stereo: true,
            });
        }
    });

    let t2 = thread::spawn(move || {
        for _ in 0..num_iterations {
            let _ = api_stop_all();
        }
    });

    t1.join().unwrap();
    t2.join().unwrap();
    
    while let Ok(_) = rx.try_recv() {} // drain commands
    
    println!("System reset vs theme start glitch stress test PASSED.");
}
