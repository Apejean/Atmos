//! OSC 주소표(osc::router). 사용자 결정(2026-10-10): 방 볼륨 OSC는 쓰지 않는다(메인 화면에서 조절). 설정의 방 볼륨
//! 주소 칸은 2026-10-24에 지웠다. 같은 주소가 여러 곳에 있으면 마지막에 등록된 하나만 반응하므로, 겹치는 주소를 찾아
//! 앱 로그에 남긴다(현장 원격 진단, HANDOFF 남은 일 5).
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::osc::router::{duplicate_osc_addresses, get_osc_action, OscAction};

fn track(id: &str, play: &str, stop: &str) -> TrackConfig {
    TrackConfig {
        id: id.into(),
        name: id.into(),
        file_path: format!("/x/{id}.wav"),
        volume: 1.0,
        is_loop: false,
        is_streaming: false,
        output_channel: 0,
        output_stereo: true,
        play_osc_address: play.into(),
        stop_osc_address: stop.into(),
    }
}

fn room(id: &str, clear: &str, tracks: Vec<TrackConfig>) -> RoomConfig {
    RoomConfig {
        id: id.into(),
        name: id.into(),
        color_hex: "#ffffff".into(),
        volume: 1.0,
        clear_osc_address: clear.into(),
        tracks,
    }
}

#[test]
fn 방_클리어와_트랙_재생_주소를_찾는다() {
    let config = AppConfig {
        rooms: vec![room("r1", "/room1/clear", vec![track("t1", "/p1", "/s1")])],
        ..AppConfig::default()
    };
    assert!(matches!(get_osc_action("/room1/clear", &config, 9_001), Some(OscAction::ClearRoom(id)) if id == "r1"));
    assert!(matches!(get_osc_action("/p1", &config, 9_001), Some(OscAction::PlayTrack(r, t)) if r == "r1" && t == "t1"));
}

#[test]
fn 겹치는_주소를_몇_곳인지와_함께_찾는다() {
    let config = AppConfig {
        theme_start_osc_address: "/theme/start".into(),
        system_reset_osc_address: String::new(),
        rooms: vec![
            room("r1", "/room/clear", vec![track("a", "/play", "/stop"), track("b", "/play", "/stop1")]),
            room("r2", "/room/clear", vec![track("c", "/play", "")]),
        ],
        ..AppConfig::default()
    };
    // 빈칸은 보지 않는다. 이름 순.
    assert_eq!(
        duplicate_osc_addresses(&config),
        vec![("/play".to_string(), 3), ("/room/clear".to_string(), 2)]
    );
}
