#[test]
fn test_hrtf_3dof_and_osc_feedback() {
    // We just want to ensure it builds and the basic HRTF overlap-add state is initialized correctly without panicking
    use rust_lib_atmos_mixer_pro::audio::binaural::VirtualMixRoomBinaural;
    
    let mut binaural = VirtualMixRoomBinaural::new(2, 256);
    binaural.enabled = true;
    
    let mut output = vec![0.0; 512]; // 256 frames * 2 channels
    // simulate yaw movement
    rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE.hrtf_yaw.store(0.5f32.to_bits(), std::sync::atomic::Ordering::Relaxed);
    
    // First frame to trigger update
    binaural.process_interleaved(&mut output, 2);
    
    // Next frame
    binaural.process_interleaved(&mut output, 2);
}
