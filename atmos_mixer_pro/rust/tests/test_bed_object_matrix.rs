use std::sync::atomic::Ordering;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::common::config::Point3D;
use rust_lib_atmos_mixer_pro::audio::player::SoundInstance;
use rust_lib_atmos_mixer_pro::audio::player::SoundData;
use std::sync::Arc;

#[test]
fn test_bed_object_matrix() {
    let out_channels = 64;
    let (gc_tx, _) = crossbeam_channel::unbounded();
    let mut mixer = AudioMixer::new(48000, out_channels, gc_tx, None);
    
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
            size: 0.5,
        }));
    }
    mixer.channel_positions = positions;
    mixer.startup_ramp.current_gain = 1.0; // bypass boot safety
    
    let mut dummy_data: Vec<f32> = vec![0.0; out_channels * 512];
    
    // --- TEST 1: BED ROUTING ---
    let sound_data = Arc::new(SoundData {
        samples: vec![1.0; 512 * 200], // give enough data to not stop playing
        channels: 1,
        sample_rate: 48000,
    });
    
    let mut bed_instance = SoundInstance::new(
        1, 1, 1, "Track 1".to_string(), 
        Some(sound_data.clone()), None, 48000, 1, false, 1.0, 0, false, None
    );
    bed_instance.fade_weight = 1.0; // bypass fade in
    
    mixer.instances[0] = Some(bed_instance);
    
    mixer.process(&mut dummy_data, out_channels);
    println!("val at 0: {}", dummy_data[0]);
    
    // Test that channel 0 has sound, channel 1 has 0
    let has_sound_0 = dummy_data[0].abs() > 0.0;
    let has_sound_1 = dummy_data[1].abs() > 0.0;
    assert!(has_sound_0, "Bed channel 0 must have sound");
    assert!(!has_sound_1, "Bed channel 1 must not bleed");
    
    // --- TEST 2: 3D OBJECT PANNING ---
    dummy_data.fill(0.0);
    mixer.instances[0] = None;
    
    let object_pos = Point3D {
        x: 10.0, y: 0.0, z: 0.0, 
        yaw_rotation: 0.0, pitch_tilt: 0.0, dispersion_angle: 0.0, size: 0.5,
    };
    
    let mut obj_instance = SoundInstance::new(
        2, 1, 2, "Track 2".to_string(), 
        Some(sound_data.clone()), None, 48000, 1, false, 1.0, usize::MAX, false, Some(object_pos)
    );
    obj_instance.fade_weight = 1.0; // bypass fade in
    
    mixer.instances[1] = Some(obj_instance);
    
    // warm up spatial interpolator
    mixer.process(&mut dummy_data, out_channels);
    println!("val at 0: {}", dummy_data[0]);
    mixer.process(&mut dummy_data, out_channels);
    println!("val at 0: {}", dummy_data[0]);
    
    let mut total_energy = 0.0;
    for &sample in &dummy_data {
        total_energy += sample.abs();
    }
    assert!(total_energy > 0.0, "Object must output energy");
}
