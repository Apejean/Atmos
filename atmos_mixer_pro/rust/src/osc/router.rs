use std::collections::HashMap;
use crate::common::config::AppConfig;
use lazy_static::lazy_static;
use std::sync::RwLock;

#[derive(Clone)]
pub enum OscAction {
    ClearRoom(String),
    SetMasterVolume(String),
    PlayTrack(String, String),
    StopTrack(String, String),
    ThemeStart(String, Vec<String>),
    SystemReset,
}

lazy_static! {
    pub static ref OSC_ROUTER_CACHE: RwLock<(u64, HashMap<String, OscAction>)> = RwLock::new((0, HashMap::new()));
}

pub fn get_osc_action(addr: &str, config: &AppConfig, config_version: u64) -> Option<OscAction> {
    // Check if cache needs updating
    {
        let cache = OSC_ROUTER_CACHE.read().unwrap();
        if cache.0 == config_version {
            return cache.1.get(addr).cloned();
        }
    }

    // Rebuild cache
    let mut new_map = HashMap::new();
    
    if !config.theme_start_osc_address.is_empty() {
        let mut track_ids = vec![];
        let mut first_room_id = String::new();
        if let Some(first_room) = config.rooms.first() {
            first_room_id = first_room.id.clone();
            track_ids = first_room.tracks.iter().filter(|t| t.is_loop).map(|t| t.id.clone()).collect();
        }
        new_map.insert(config.theme_start_osc_address.clone(), OscAction::ThemeStart(first_room_id, track_ids));
    }
    
    if !config.system_reset_osc_address.is_empty() {
        new_map.insert(config.system_reset_osc_address.clone(), OscAction::SystemReset);
    }

    for room in &config.rooms {
        if !room.clear_osc_address.is_empty() {
            new_map.insert(room.clear_osc_address.clone(), OscAction::ClearRoom(room.id.clone()));
        }
        if !room.volume_osc_address.is_empty() {
            new_map.insert(room.volume_osc_address.clone(), OscAction::SetMasterVolume(room.id.clone()));
        }
        for track in &room.tracks {
            if !track.play_osc_address.is_empty() {
                new_map.insert(track.play_osc_address.clone(), OscAction::PlayTrack(room.id.clone(), track.id.clone()));
            }
            if !track.stop_osc_address.is_empty() {
                new_map.insert(track.stop_osc_address.clone(), OscAction::StopTrack(room.id.clone(), track.id.clone()));
            }
        }
    }

    let action = new_map.get(addr).cloned();
    let mut cache_write = OSC_ROUTER_CACHE.write().unwrap();
    *cache_write = (config_version, new_map);
    action
}
