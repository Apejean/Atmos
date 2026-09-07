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

/// 실측 SOFA(MIT KEMAR) HRIR을 사용한 바이노럴 렌더링이 azimuth에 따라
/// 실제로 좌/우 채널 ITD/ILD를 변화시키는지 검증한다(더미 스텁 시절과 달리).
fn measure_lr_rms_for_azimuth(azimuth_deg: f32) -> (f32, f32) {
    use rust_lib_atmos_mixer_pro::audio::binaural::VirtualMixRoomBinaural;
    use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
    use std::sync::atomic::Ordering;

    let block_size = 256;
    let mut binaural = VirtualMixRoomBinaural::new(1, block_size);
    binaural.enabled = true;

    // azimuth_deg = -yaw.to_degrees() 이므로 yaw = -azimuth_deg
    let yaw_rad = (-azimuth_deg).to_radians();
    GLOBAL_STATE.hrtf_yaw.store(yaw_rad.to_bits(), Ordering::Relaxed);

    // 무음 블록을 여러 번 흘려 crossfade가 목표 HRIR로 완전히 정착하도록 한다.
    let mut silent = vec![0.0f32; block_size * 2];
    for _ in 0..4 {
        binaural.process_interleaved(&mut silent, 2);
    }

    // 1kHz 사인파를 입력해 정착된 HRIR과 컨볼루션된 결과를 관측한다.
    let sample_rate = 44100.0f32;
    let mut rms_l_sq = 0.0f32;
    let mut rms_r_sq = 0.0f32;
    let mut count = 0usize;
    for block in 0..6 {
        let mut buf = vec![0.0f32; block_size * 2];
        for frame in 0..block_size {
            let global_frame = block * block_size + frame;
            let t = global_frame as f32 / sample_rate;
            let sample = (2.0 * std::f32::consts::PI * 1000.0 * t).sin();
            buf[frame * 2] = sample; // mono source channel
        }
        binaural.process_interleaved(&mut buf, 2);
        for frame in 0..block_size {
            let l = buf[frame * 2];
            let r = buf[frame * 2 + 1];
            rms_l_sq += l * l;
            rms_r_sq += r * r;
            count += 1;
        }
    }

    (
        (rms_l_sq / count as f32).sqrt(),
        (rms_r_sq / count as f32).sqrt(),
    )
}

#[test]
fn test_zero_defect_real_sofa_hrtf_ild_varies_with_azimuth() {
    let (rms_l_left90, rms_r_left90) = measure_lr_rms_for_azimuth(-90.0);
    let (rms_l_center, rms_r_center) = measure_lr_rms_for_azimuth(0.0);
    let (rms_l_right90, rms_r_right90) = measure_lr_rms_for_azimuth(90.0);

    println!(
        "Azimuth -90: L={:.5} R={:.5} | Azimuth 0: L={:.5} R={:.5} | Azimuth +90: L={:.5} R={:.5}",
        rms_l_left90, rms_r_left90, rms_l_center, rms_r_center, rms_l_right90, rms_r_right90
    );

    // 실측 HRIR을 사용하면 좌/우 채널 에너지가 절대 0이 아니어야 한다(더미 시절 컨볼루션과 구분).
    assert!(rms_l_left90 > 1e-6 && rms_r_left90 > 1e-6);
    assert!(rms_l_right90 > 1e-6 && rms_r_right90 > 1e-6);

    // ILD(Interaural Level Difference): -90도와 +90도에서 우세한 귀(ear)가 서로 반대여야 한다.
    let dominant_left_at_minus90 = rms_l_left90 > rms_r_left90;
    let dominant_left_at_plus90 = rms_l_right90 > rms_r_right90;
    assert_ne!(
        dominant_left_at_minus90, dominant_left_at_plus90,
        "azimuth -90/+90 에서 우세 채널(ILD)이 동일함 — 실측 HRIR이 반영되지 않음"
    );

    // 정면(0도)은 좌우 대칭에 가까워야 한다(과도한 편향 금지).
    let center_ratio = rms_l_center / rms_r_center.max(1e-6);
    assert!(
        center_ratio > 0.5 && center_ratio < 2.0,
        "정면 azimuth에서 좌우 에너지가 과도하게 비대칭임: ratio={}",
        center_ratio
    );
}
