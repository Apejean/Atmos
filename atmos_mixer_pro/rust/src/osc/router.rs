use std::collections::HashMap;
use crate::common::config::AppConfig;
use lazy_static::lazy_static;
use std::sync::RwLock;

/// 방 볼륨 OSC는 쓰지 않는다(사용자 결정 2026-10-10, 메인 화면에서 조절). 설정의 `volume_osc_address`는 호환을 위해
/// 남아 있지만 주소표에 넣지 않는다.
#[derive(Clone)]
pub enum OscAction {
    ClearRoom(String),
    PlayTrack(String, String),
    StopTrack(String, String),
    ThemeStart,
    SystemReset,
}

lazy_static! {
    pub static ref OSC_ROUTER_CACHE: RwLock<(u64, HashMap<String, OscAction>)> = RwLock::new((0, HashMap::new()));
}

/// 두 곳 이상에서 쓰는 주소와 그 수(이름 순). 이 주소로는 마지막에 등록된 하나만 반응한다. 빈칸과 듣지 않는 방
/// 볼륨 주소는 보지 않는다.
pub fn duplicate_osc_addresses(config: &AppConfig) -> Vec<(String, usize)> {
    let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    let rooms = config.rooms.iter().flat_map(|room| {
        std::iter::once(room.clear_osc_address.as_str()).chain(
            room.tracks
                .iter()
                .flat_map(|track| [track.play_osc_address.as_str(), track.stop_osc_address.as_str()]),
        )
    });
    for address in [config.theme_start_osc_address.as_str(), config.system_reset_osc_address.as_str()]
        .into_iter()
        .chain(rooms)
        .filter(|address| !address.is_empty())
    {
        *counts.entry(address).or_default() += 1;
    }
    counts.into_iter().filter(|(_, n)| *n > 1).map(|(address, n)| (address.to_string(), n)).collect()
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
        new_map.insert(config.theme_start_osc_address.clone(), OscAction::ThemeStart);
    }
    
    if !config.system_reset_osc_address.is_empty() {
        new_map.insert(config.system_reset_osc_address.clone(), OscAction::SystemReset);
    }

    for room in &config.rooms {
        if !room.clear_osc_address.is_empty() {
            new_map.insert(room.clear_osc_address.clone(), OscAction::ClearRoom(room.id.clone()));
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

    // 겹치는 주소는 마지막 것만 반응한다. 현장 로그로 원인을 알 수 있게 남긴다(설정이 바뀔 때 한 번).
    for (address, n) in duplicate_osc_addresses(config) {
        crate::core::state::GLOBAL_STATE.log(format!(
            "OSC 주소가 겹친다: '{address}'를 {n}곳이 쓴다 — 마지막에 등록된 것만 반응한다"
        ));
    }

    let action = new_map.get(addr).cloned();
    let mut cache_write = OSC_ROUTER_CACHE.write().unwrap();
    *cache_write = (config_version, new_map);
    action
}
