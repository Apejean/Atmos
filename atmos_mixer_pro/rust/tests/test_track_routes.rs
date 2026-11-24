//! 매트릭스 라우팅(설계 `docs/superpowers/specs/2026-11-14-matrix-routing-design.md` 개정 3) — 설정 구조.
//! 옛 설정은 그대로 읽히고(표 없음·트랙별 출력 매핑), 표를 쓰지 않는 설정은 저장 모양이 지금과 같으며,
//! 방식을 바꿔도 두 방식의 설정이 서로를 건드리지 않는다(D13·D14).

use rust_lib_atmos_mixer_pro::common::config::{AppConfig, OutputRoutingMode, RoomConfig, TrackConfig, TrackRoute};

fn track(id: &str, out: u32, stereo: bool, routes: Vec<TrackRoute>) -> TrackConfig {
    TrackConfig {
        id: id.to_string(),
        name: id.to_string(),
        file_path: format!("D:/audio/{id}.wav"),
        volume: 0.8,
        is_loop: true,
        is_streaming: true,
        output_channel: out,
        output_stereo: stereo,
        play_osc_address: String::new(),
        stop_osc_address: String::new(),
        routes,
    }
}

fn route(file_channel: u32, output_channel: u32, gain: f32) -> TrackRoute {
    TrackRoute { file_channel, output_channel, gain }
}

fn config(mode: OutputRoutingMode, tracks: Vec<TrackConfig>) -> AppConfig {
    AppConfig {
        output_routing_mode: mode,
        rooms: vec![RoomConfig {
            id: "room_1".to_string(),
            name: "방 1".to_string(),
            color_hex: "#1565C0".to_string(),
            volume: 1.0,
            clear_osc_address: String::new(),
            tracks,
        }],
        ..AppConfig::default()
    }
}

fn round_trip(c: &AppConfig) -> AppConfig {
    serde_json::from_str(&serde_json::to_string(c).unwrap()).unwrap()
}

#[test]
fn an_old_config_reads_as_track_mapping_without_tables() {
    // 1.3.18이 쓴 모양(새 키 없음).
    let json = r##"{
        "osc_port": 8000, "device_name": null, "buffer_size": 1024,
        "mono_configs": {}, "stereo_configs": {}, "multi_configs": {},
        "rooms": [{"id": "r", "name": "r", "color_hex": "#000000", "volume": 1.0,
                   "tracks": [{"id": "t", "name": "t", "file_path": "a.wav", "volume": 1.0, "is_loop": true,
                               "output_channel": 2, "output_stereo": false}]}],
        "is_exhibition_mode": false
    }"##;
    let c: AppConfig = serde_json::from_str(json).unwrap();
    assert_eq!(c.output_routing_mode, OutputRoutingMode::TrackMapping);
    let t = &c.rooms[0].tracks[0];
    assert!(t.routes.is_empty());
    assert_eq!((t.output_channel, t.output_stereo), (2, false));
}

#[test]
fn a_config_without_tables_saves_the_same_keys_as_before() {
    let c = config(OutputRoutingMode::TrackMapping, vec![track("t1", 0, true, vec![])]);
    let json = serde_json::to_string(&c).unwrap();
    assert!(!json.contains("output_routing_mode"), "기본 방식은 키를 쓰지 않는다: {json}");
    assert!(!json.contains("\"routes\""), "빈 표는 키를 쓰지 않는다: {json}");
}

#[test]
fn a_matrix_config_survives_save_and_load() {
    let c = config(
        OutputRoutingMode::Matrix,
        vec![
            track("mono", 0, false, vec![route(0, 0, 1.0), route(0, 2, 0.7), route(0, 4, 0.5)]),
            track("plain", 6, true, vec![]),
        ],
    );
    let json = serde_json::to_string(&c).unwrap();
    assert!(json.contains("\"output_routing_mode\":\"matrix\""), "{json}");
    let back = round_trip(&c);
    assert_eq!(back.output_routing_mode, OutputRoutingMode::Matrix);
    assert_eq!(back.rooms[0].tracks[0].routes, c.rooms[0].tracks[0].routes);
    assert!(back.rooms[0].tracks[1].routes.is_empty());
}

#[test]
fn a_route_without_gain_reads_as_full_volume() {
    let r: TrackRoute = serde_json::from_str(r#"{"file_channel": 1, "output_channel": 3}"#).unwrap();
    assert_eq!(r, route(1, 3, 1.0));
}

#[test]
fn switching_modes_keeps_both_settings() {
    let original = config(
        OutputRoutingMode::Matrix,
        vec![track("a", 4, true, vec![route(0, 1, 1.0), route(1, 0, 0.25)])],
    );
    let mut c = original.clone();
    for mode in [OutputRoutingMode::TrackMapping, OutputRoutingMode::Matrix, OutputRoutingMode::TrackMapping] {
        c.output_routing_mode = mode;
        c = round_trip(&c);
        let (t, o) = (&c.rooms[0].tracks[0], &original.rooms[0].tracks[0]);
        assert_eq!(t.routes, o.routes, "{mode:?}");
        assert_eq!((t.output_channel, t.output_stereo), (o.output_channel, o.output_stereo), "{mode:?}");
    }
}

// ---- 2단계: 표 검증(audio::route::resolve_routes, 설계 6.1) ----

use rust_lib_atmos_mixer_pro::audio::route::{resolve_routes, RouteEntry, MAX_TRACK_ROUTES};

fn entry(src: u16, out: u16, gain: f32) -> RouteEntry {
    RouteEntry { src, out, gain }
}

#[test]
fn bad_gains_are_dropped_and_loud_ones_are_capped() {
    let r = resolve_routes(
        &[
            route(0, 0, 0.0),
            route(0, 1, -0.5),
            route(0, 2, f32::NAN),
            route(0, 3, f32::INFINITY),
            route(0, 4, 1.5),
            route(0, 5, 0.25),
        ],
        Some(1),
    );
    assert_eq!(r.entries, vec![entry(0, 4, 1.0), entry(0, 5, 0.25)]);
    assert_eq!(r.dropped_bad_gain, 4);
    assert_eq!(r.dropped(), 4);
}

#[test]
fn channels_outside_the_file_or_the_gate_table_are_dropped() {
    let r = resolve_routes(&[route(0, 0, 1.0), route(1, 1, 1.0), route(2, 2, 1.0), route(0, 4096, 1.0)], Some(2));
    assert_eq!(r.entries, vec![entry(0, 0, 1.0), entry(1, 1, 1.0)]);
    assert_eq!((r.dropped_file_channel, r.dropped_output), (1, 1));
    // 파일 채널 수를 모르면 파일 채널 상한(256)까지 둔다 — 콜백이 실제 채널 수로 다시 거른다.
    let r = resolve_routes(&[route(255, 0, 1.0), route(256, 0, 1.0)], None);
    assert_eq!(r.entries, vec![entry(255, 0, 1.0)]);
    assert_eq!(r.dropped_file_channel, 1);
}

#[test]
fn a_repeated_cell_keeps_the_first_and_cells_are_sorted_by_output_then_file() {
    let r = resolve_routes(
        &[route(1, 5, 0.5), route(0, 5, 1.0), route(0, 2, 0.7), route(1, 5, 0.9), route(3, 0, 1.0)],
        Some(4),
    );
    assert_eq!(r.entries, vec![entry(3, 0, 1.0), entry(0, 2, 0.7), entry(0, 5, 1.0), entry(1, 5, 0.5)]);
    assert_eq!(r.dropped_duplicate, 1);
}

#[test]
fn more_than_the_cell_limit_keeps_the_lowest_outputs() {
    let routes: Vec<TrackRoute> = (0..600u32).rev().map(|o| route(0, o, 1.0)).collect();
    let r = resolve_routes(&routes, Some(1));
    assert_eq!(r.entries.len(), MAX_TRACK_ROUTES);
    assert_eq!(r.dropped_over_limit, 600 - MAX_TRACK_ROUTES);
    assert_eq!(r.entries[0], entry(0, 0, 1.0));
    assert_eq!(r.entries[MAX_TRACK_ROUTES - 1], entry(0, (MAX_TRACK_ROUTES - 1) as u16, 1.0));
}

// ---- 개정 3 ②: 표는 처리 폭을 바꾸지 않는다(폭은 출력 설정만으로 정해진다) ----

#[test]
fn tables_do_not_change_the_processing_width() {
    use rust_lib_atmos_mixer_pro::core::processing_channels::processing_channel_count;
    let row: rust_lib_atmos_mixer_pro::common::config::ChannelSetting = serde_json::from_str(
        r#"{"enabled": true, "custom_name": "", "delay_ms": 0.0, "eq_bands": [], "position": null, "phase_invert": false, "gain_db": 0.0}"#,
    )
    .unwrap();
    let mut mapping = config(OutputRoutingMode::TrackMapping, vec![track("a", 0, true, vec![])]);
    mapping.stereo_configs.insert(1, row);
    let base = processing_channel_count(&mapping, 94);
    let mut matrix = mapping.clone();
    matrix.output_routing_mode = OutputRoutingMode::Matrix;
    matrix.rooms[0].tracks[0].routes = vec![route(0, 0, 1.0), route(0, 19, 1.0), route(0, 60, 1.0)];
    assert_eq!(processing_channel_count(&matrix, 94), base, "표가 가리키는 출력 때문에 처리 폭이 바뀌었다");
    assert_eq!(base, 2, "스테레오 1/2만 켰을 때 처리 폭");
}
