use std::sync::atomic::Ordering;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::common::config::Point3D;
use rust_lib_atmos_mixer_pro::audio::commands::AudioCommand;

#[test]
fn test_bed_object_matrix() {
    // 64-channel matrix simulation
    let out_channels = 64;
    let mut mixer = AudioMixer::new(48000.0, out_channels);
    
    // Enable all 64 channels
    for ch in 0..out_channels {
        GLOBAL_STATE.enabled_channels[ch].store(true, Ordering::SeqCst);
    }
    
    // Set positions for all 64 speakers in a circle
    let mut positions = Vec::new();
    for i in 0..out_channels {
        let angle = (i as f32 / out_channels as f32) * std::f32::consts::PI * 2.0;
        positions.push(Some(Point3D {
            x: angle.cos() * 10.0,
            y: angle.sin() * 10.0,
            z: 0.0,
            yaw_rotation: 0.0,
            pitch_tilt: 0.0,
            dispersion_angle: 0.5,
        }));
    }
    mixer.update_channel_positions(positions);
    
    let mut dummy_data: Vec<f32> = vec![0.0; out_channels * 512];
    
    // --- TEST 1: BED ROUTING ---
    // Inject a track routed specifically to Ch 1 (index 0)
    let bed_instance = rust_lib_atmos_mixer_pro::audio::player::SoundInstance::new(
        "bed1".to_string(), "room1".to_string(), "t1".to_string(), "Track 1".to_string(), 
        vec![1.0; 512], None, 48000, 1, false, 1.0, 1.0, 0, false, None
    );
    mixer.add_instance(bed_instance);
    
    mixer.process(&mut dummy_data, out_channels);
    
    // --- TEST 2: 3D OBJECT PANNING ---
    dummy_data.fill(0.0);
    
    let object_pos = Point3D {
        x: 10.0, y: 0.0, z: 0.0, 
        yaw_rotation: 0.0, pitch_tilt: 0.0, dispersion_angle: 0.0,
    };
    let obj_instance = rust_lib_atmos_mixer_pro::audio::player::SoundInstance::new(
        "obj1".to_string(), "room1".to_string(), "t2".to_string(), "Track 2".to_string(), 
        vec![1.0; 512], None, 48000, 1, false, 1.0, 1.0, usize::MAX, false, Some(object_pos)
    );
    
    mixer.add_instance(obj_instance);
    for _ in 0..200 {
        mixer.process(&mut dummy_data, out_channels);
    }
}
