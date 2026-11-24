//! 매트릭스 라우팅(설계 개정 3) — 재생 시작·API·설정 교체 때 경로 맞추기(설계 6.6, 8.2).
//! 설정을 넣고 API를 부른 뒤 명령 큐를 꺼내 본다. 전역 상태를 쓰므로 이 파일의 시험은 하나씩 돈다.

use rust_lib_atmos_mixer_pro::api::simple::{api_play_track, api_save_config, api_set_track_output, api_set_track_routes};
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, OutputRoutingMode, RoomConfig, TrackConfig, TrackRoute};
use rust_lib_atmos_mixer_pro::common::utils::hash_id;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());

fn dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("atmos_play_track_routes_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// 2채널 16bit 0.2초 WAV.
fn stereo_wav(path: &Path) {
    let spec = hound::WavSpec { channels: 2, sample_rate: 48_000, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for i in 0..9_600 {
        let v = ((i as f32 * 0.05).sin() * 3000.0) as i16;
        w.write_sample(v).unwrap();
        w.write_sample(v / 2).unwrap();
    }
    w.finalize().unwrap();
}

fn route(file_channel: u32, output_channel: u32) -> TrackRoute {
    TrackRoute { file_channel, output_channel, gain: 1.0 }
}

fn track(id: &str, path: &Path, is_loop: bool, routes: Vec<TrackRoute>) -> TrackConfig {
    TrackConfig {
        id: id.to_string(),
        name: id.to_string(),
        file_path: path.to_string_lossy().into_owned(),
        volume: 1.0,
        is_loop,
        is_streaming: is_loop,
        output_channel: 4,
        output_stereo: true,
        play_osc_address: String::new(),
        stop_osc_address: String::new(),
        routes,
    }
}

fn config(mode: OutputRoutingMode, tracks: Vec<TrackConfig>) -> AppConfig {
    AppConfig {
        output_routing_mode: mode,
        rooms: vec![RoomConfig {
            id: "room_r".to_string(),
            name: "방".to_string(),
            color_hex: "#000000".to_string(),
            volume: 1.0,
            clear_osc_address: String::new(),
            tracks,
        }],
        ..AppConfig::default()
    }
}

fn install(c: AppConfig) {
    GLOBAL_STATE.engine_sample_rate.store(48_000, Ordering::SeqCst);
    *GLOBAL_STATE.config.write().unwrap() = Some(c);
    GLOBAL_STATE.clear_playing_tracks();
    drain();
}

fn drain() -> Vec<AudioCommand> {
    let mut v = Vec::new();
    while let Ok(c) = GLOBAL_STATE.command_receiver.try_recv() {
        v.push(c);
    }
    v
}

/// 큐에서 재생 명령의 (표 경로인가, 칸 (src, out)).
fn played(cmds: &[AudioCommand]) -> Vec<(bool, Vec<(u16, u16)>)> {
    cmds.iter()
        .filter_map(|c| match c {
            AudioCommand::PlayTrack { instance, .. } => {
                Some((instance.table_active, instance.routes.iter().map(|e| (e.src, e.out)).collect()))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn a_matrix_track_starts_on_its_table_with_the_real_file_channel_count() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let wav = dir().join("st.wav");
    stereo_wav(&wav);
    // 파일 채널 3번 칸은 2채널 파일에 없으니 빠진다. 스트리밍(루프)·메모리(단발) 두 길 모두.
    let cells = vec![route(0, 0), route(1, 2), route(2, 1)];
    for is_loop in [true, false] {
        install(config(OutputRoutingMode::Matrix, vec![track("t", &wav, is_loop, cells.clone())]));
        api_play_track("room_r".into(), "t".into()).unwrap();
        let got = played(&drain());
        assert_eq!(got, vec![(true, vec![(0, 0), (1, 2)])], "is_loop={is_loop}");
    }
}

#[test]
fn in_track_mapping_mode_a_saved_table_is_not_used() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let wav = dir().join("st2.wav");
    stereo_wav(&wav);
    for is_loop in [true, false] {
        install(config(OutputRoutingMode::TrackMapping, vec![track("t", &wav, is_loop, vec![route(0, 0)])]));
        api_play_track("room_r".into(), "t".into()).unwrap();
        assert_eq!(played(&drain()), vec![(false, vec![])], "is_loop={is_loop}");
    }
}

#[test]
fn the_routes_api_only_works_in_matrix_mode_and_with_cells() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let wav = dir().join("st3.wav");
    stereo_wav(&wav);
    install(config(OutputRoutingMode::TrackMapping, vec![track("t", &wav, true, vec![])]));
    assert!(api_set_track_routes("room_r".into(), "t".into(), vec![route(0, 1)]).is_err());
    assert!(drain().is_empty(), "매핑 방식에서 표 명령이 나갔다");

    install(config(OutputRoutingMode::Matrix, vec![track("t", &wav, true, vec![route(0, 0)])]));
    assert!(api_set_track_routes("room_r".into(), "t".into(), vec![]).is_err(), "빈 표는 오류");
    assert!(drain().is_empty());
    api_set_track_routes("room_r".into(), "t".into(), vec![route(1, 3), route(0, 2)]).unwrap();
    let cmds = drain();
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        AudioCommand::SetTrackRoutes { room_id, track_id, entries } => {
            assert_eq!((*room_id, *track_id), (hash_id("room_r"), hash_id("t")));
            assert_eq!(entries.iter().map(|e| (e.src, e.out)).collect::<Vec<_>>(), vec![(0, 2), (1, 3)]);
        }
        _ => panic!("SetTrackRoutes가 아니다"),
    }

    // 매트릭스이고 표가 있는 트랙에 매핑 출력 변경은 보내지 않는다(두 방식이 섞이지 않게).
    api_set_track_output("room_r".into(), "t".into(), 6, true).unwrap();
    assert!(drain().is_empty(), "표 트랙에 매핑 경로 명령이 나갔다");
}

fn route_commands(cmds: &[AudioCommand]) -> Vec<(u32, &'static str)> {
    cmds.iter()
        .filter_map(|c| match c {
            AudioCommand::SetTrackRoutes { track_id, .. } => Some((*track_id, "routes")),
            AudioCommand::SetTrackOutput { track_id, .. } => Some((*track_id, "output")),
            _ => None,
        })
        .collect()
}

#[test]
fn saving_a_config_resyncs_the_paths_that_changed() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let wav = dir().join("st4.wav");
    stereo_wav(&wav);
    let file = dir().join("config.json").to_string_lossy().into_owned();
    let tracks = vec![track("a", &wav, true, vec![route(0, 0)]), track("b", &wav, true, vec![])];
    install(config(OutputRoutingMode::TrackMapping, tracks.clone()));

    // 매핑 → 매트릭스: 모든 트랙(표가 있으면 표, 없으면 매핑).
    api_save_config(file.clone(), config(OutputRoutingMode::Matrix, tracks.clone())).unwrap();
    let mut got = route_commands(&drain());
    got.sort();
    let mut want = vec![(hash_id("a"), "routes"), (hash_id("b"), "output")];
    want.sort();
    assert_eq!(got, want, "방식을 바꾸면 모든 트랙에 새 방식의 경로가 가야 한다");

    // 같은 설정을 다시 저장하면 아무것도 보내지 않는다.
    api_save_config(file.clone(), config(OutputRoutingMode::Matrix, tracks.clone())).unwrap();
    assert!(route_commands(&drain()).is_empty(), "바뀐 것이 없는데 경로 명령이 나갔다");

    // 매트릭스에서 한 트랙의 표만 바꾸면 그 트랙에만.
    let mut changed = tracks.clone();
    changed[0].routes = vec![route(0, 1), route(1, 3)];
    api_save_config(file.clone(), config(OutputRoutingMode::Matrix, changed.clone())).unwrap();
    assert_eq!(route_commands(&drain()), vec![(hash_id("a"), "routes")]);

    // 매트릭스 → 매핑: 모든 트랙이 매핑 경로로.
    api_save_config(file.clone(), config(OutputRoutingMode::TrackMapping, changed.clone())).unwrap();
    let got = route_commands(&drain());
    assert_eq!(got.len(), 2);
    assert!(got.iter().all(|(_, k)| *k == "output"), "{got:?}");

    // 매핑 방식에서 매핑 값만 바꾸면 예전처럼 보내지 않는다.
    let mut moved = changed.clone();
    moved[1].output_channel = 8;
    api_save_config(file, config(OutputRoutingMode::TrackMapping, moved)).unwrap();
    assert!(route_commands(&drain()).is_empty());
}

/// 설정을 통째로 바꾸는 곳은 모두 경로 맞추기를 지나야 한다(새로 생기면 이 시험이 실패한다, 설계 8.2 소스 감사).
#[test]
fn every_whole_config_replacement_resyncs_routing() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut sites = 0;
    for entry in walk(&src) {
        let text = std::fs::read_to_string(&entry).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let l = line.trim();
            if l.starts_with("//") {
                continue;
            }
            let replaces = (l.starts_with("*global_config = Some(") || l.starts_with("*state = Some("))
                && l.ends_with(");");
            if replaces {
                sites += 1;
                let next: String = lines[i + 1..(i + 4).min(lines.len())].join("\n");
                assert!(
                    next.contains("resync_routing("),
                    "{}:{} 설정을 통째로 바꾸고 경로를 맞추지 않는다: {l}",
                    entry.display(),
                    i + 1
                );
            }
        }
    }
    assert_eq!(sites, 4, "설정을 통째로 바꾸는 곳 수가 달라졌다(api_get_config·api_save_config·api_preload_all_sounds·장면 불러오기)");
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
    out
}
