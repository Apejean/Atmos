use crate::common::config::Point3D;
use crate::core::state::GLOBAL_STATE;
use crate::common::commands::AudioCommand;
use std::collections::HashSet;

/// DBAP (Distance-Based Amplitude Panning) 기반 3D 음압 히트맵 연산
/// Inverse Square Law 롤오프를 사용하여 그리드 상의 각 포인트에 대한 총 음압을 0.0 ~ 1.0으로 정규화하여 반환합니다.
#[flutter_rust_bridge::frb(sync)]
pub fn api_calculate_dbap_heatmap(
    speakers: Vec<Point3D>, 
    room_width: f32, 
    room_depth: f32, 
    grid_size: usize,
    rolloff_factor: f32
) -> Vec<f32> {
    let mut matrix = vec![0.0; grid_size * grid_size];
    if speakers.is_empty() {
        return matrix;
    }
    
    let epsilon = 1e-6;
    let mut max_intensity: f32 = 0.0;

    for i in 0..grid_size {
        for j in 0..grid_size {
            // (i, j) 픽셀을 3D 공간 (x, z) 룸 좌표로 매핑 (-W/2 ~ W/2, -D/2 ~ D/2)
            let x = (j as f32 / (grid_size.saturating_sub(1).max(1) as f32)) * room_width - (room_width / 2.0);
            let z = (i as f32 / (grid_size.saturating_sub(1).max(1) as f32)) * room_depth - (room_depth / 2.0);
            
            // 바닥(히트맵) 높이는 y = 0.0으로 가정
            let mut total_weight = 0.0;
            
            for spk in &speakers {
                let dx = x - spk.x;
                let dy = 0.0 - spk.y;
                let dz = z - spk.z;
                
                // 유클리드 거리 d_i
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                
                // 역제곱 법칙 등에 의한 Weight = 1 / (d_i ^ a + ϵ)
                let weight = 1.0 / (dist.powf(rolloff_factor) + epsilon);
                total_weight += weight;
            }
            
            if total_weight > max_intensity {
                max_intensity = total_weight;
            }
            
            matrix[i * grid_size + j] = total_weight;
        }
    }
    
    // 정규화 (0.0 ~ 1.0)
    if max_intensity > 0.0 {
        for val in matrix.iter_mut() {
            *val /= max_intensity;
        }
    }
    
    matrix
}

/// 새로운 스피커 생성 시 사용 중이지 않은 가장 빠른 채널 번호(1번부터 스캔)를 자동 할당하여 반환합니다.
#[flutter_rust_bridge::frb(sync)]
pub fn api_allocate_next_speaker_channel() -> u32 {
    let config = {
        let state = GLOBAL_STATE.config.read().unwrap();
        state.clone().unwrap_or_default()
    };
    
    let mut used_channels = HashSet::new();
    
    for (&ch, _) in &config.mono_configs {
        used_channels.insert(ch);
    }
    for (&ch, _) in &config.stereo_configs {
        used_channels.insert(ch);
    }
    for (&ch, _) in &config.multi_configs {
        used_channels.insert(ch);
    }
    
    let mut next_channel = 1;
    loop {
        if !used_channels.contains(&next_channel) {
            return next_channel;
        }
        next_channel += 1;
    }
}

/// 글로벌 리버브 믹스(Reverb Mix) 및 감쇠 시간(Reverb Decay)을 설정합니다.
#[flutter_rust_bridge::frb(sync)]
pub fn api_set_global_reverb(mix: f32, decay: f32) {
    {
        let mut state = GLOBAL_STATE.config.write().unwrap();
        if let Some(config) = state.as_mut() {
            config.global_reverb_mix = mix;
            config.global_reverb_decay = decay;
        }
    }
    
    let _ = GLOBAL_STATE.command_sender.send(AudioCommand::SetReverbParams { mix, decay });
}

/// 특정 룸 존의 차음 성능(Transmission Loss, dB)을 설정합니다.
#[flutter_rust_bridge::frb(sync)]
pub fn api_set_transmission_loss(room_zone_id: u32, loss_db: f32) {
    {
        let mut state = GLOBAL_STATE.config.write().unwrap();
        if let Some(config) = state.as_mut() {
            for zone in &mut config.room_zones {
                if zone.room_id == room_zone_id {
                    zone.transmission_loss_db = loss_db;
                }
            }
        }
    }
    
    // Frontend should call api_update_spatial_config_json to update the DSP engine.
}
