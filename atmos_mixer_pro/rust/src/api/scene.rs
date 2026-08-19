use crate::api::error::AtmosError;
use std::fs;
use std::path::Path;

pub fn save_scene(scene_id: &str, name: &str) -> Result<(), AtmosError> {
    println!("✅ [디버깅] 씬 저장 실제 로직 수행: {} ({})", scene_id, name);
    
    let scenes_dir = Path::new("scenes");
    if !scenes_dir.exists() {
        fs::create_dir_all(scenes_dir).map_err(|e| AtmosError::IoError(e.to_string()))?;
    }
    
    let file_path = scenes_dir.join(format!("{}.json", scene_id));
    
    // For now, we will save the current GLOBAL_STATE.config into a JSON file
    let config = {
        let state = crate::core::state::GLOBAL_STATE.config.read().unwrap();
        state.clone()
    };
    
    let json_data = serde_json::to_string_pretty(&config)
        .map_err(|e| AtmosError::SerializationError(e.to_string()))?;
        
    fs::write(&file_path, json_data)
        .map_err(|e| AtmosError::IoError(e.to_string()))?;
        
    println!("✅ [디버깅] 씬 저장 완료: {:?}", file_path);
    Ok(())
}

pub fn load_scene(scene_id: &str) -> Result<(), AtmosError> {
    println!("✅ [디버깅] 씬 불러오기: {}", scene_id);
    
    let file_path = Path::new("scenes").join(format!("{}.json", scene_id));
    if !file_path.exists() {
        return Err(AtmosError::IoError("Scene file not found".to_string()));
    }
    
    let json_data = fs::read_to_string(&file_path)
        .map_err(|e| AtmosError::IoError(e.to_string()))?;
        
    let new_config: crate::common::config::AppConfig = serde_json::from_str(&json_data)
        .map_err(|e| AtmosError::SerializationError(e.to_string()))?;
        
    {
        let mut state = crate::core::state::GLOBAL_STATE.config.write().unwrap();
        *state = new_config.clone();
    }
    
    // Notify audio engine to update config
    let _ = crate::core::state::GLOBAL_STATE.command_sender.send(crate::common::commands::AudioCommand::UpdateSpatialConfig {
        channel_positions: new_config.mono_configs.iter().map(|(&id, c)| (id, c.position.clone())).collect(),
        room_zones: new_config.room_zones.clone(),
        trajectory: new_config.global_trajectory.clone(),
    });
    
    Ok(())
}

pub fn clear_room() -> Result<(), AtmosError> {
    println!("✅ [디버깅] 룸 정보 초기화 (Clear Room)");
    
    {
        let mut state = crate::core::state::GLOBAL_STATE.config.write().unwrap();
        state.rooms.clear();
        state.room_zones.clear();
        state.global_trajectory = None;
    }
    
    let _ = crate::core::state::GLOBAL_STATE.command_sender.send(crate::common::commands::AudioCommand::UpdateSpatialConfig {
        channel_positions: std::collections::HashMap::new(),
        room_zones: Vec::new(),
        trajectory: None,
    });
    
    Ok(())
}
