//! 프로젝트 파일을 다른 PC에서 열 때 트랙 오디오·도면 경로 다시 연결(core::media_relink).
//! 실제 파일 시스템(임시 폴더)으로 확인한다. 전역 상태를 쓰지 않는다.
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::media_relink::{file_name_of, relink_tracks, MediaFinder};
use std::path::{Path, PathBuf};

fn fresh_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("atmos_relink_test_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// [root] 아래에 빈 파일을 만들고 경로를 돌려준다. [relative]의 `/`는 그 OS의 구분자로 잇는다
/// (다시 연결된 경로는 폴더를 훑어 찾으므로 Windows에서는 `\`로 돌아온다).
fn touch(root: &Path, relative: &str) -> PathBuf {
    let path = relative.split('/').fold(root.to_path_buf(), |p, part| p.join(part));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"").unwrap();
    path
}

fn track(id: &str, file_path: &str) -> TrackConfig {
    TrackConfig {
        id: id.into(),
        name: id.into(),
        file_path: file_path.into(),
        volume: 0.8,
        is_loop: true,
        is_streaming: false,
        output_channel: 2,
        output_stereo: false,
        play_osc_address: format!("/{id}/play"),
        stop_osc_address: format!("/{id}/stop"),
    }
}

fn config_with(paths: &[(&str, &str)]) -> AppConfig {
    AppConfig {
        osc_port: 9001,
        rooms: vec![RoomConfig {
            id: "r1".into(),
            name: "방 1".into(),
            color_hex: "#123456".into(),
            volume: 0.7,
            volume_osc_address: "/r1/volume".into(),
            clear_osc_address: "/r1/clear".into(),
            tracks: paths.iter().map(|(id, p)| track(id, p)).collect(),
        }],
        ..AppConfig::default()
    }
}

fn paths(config: &AppConfig) -> Vec<String> {
    config.rooms.iter().flat_map(|r| r.tracks.iter().map(|t| t.file_path.clone())).collect()
}

fn s(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[test]
fn 경로의_마지막_이름은_슬래시와_역슬래시를_모두_구분자로_본다() {
    assert_eq!(file_name_of("/Users/a/Exhibit/bgm.wav"), Some("bgm.wav"));
    assert_eq!(file_name_of(r"C:\Shows\Exhibit\bgm.wav"), Some("bgm.wav"));
    assert_eq!(file_name_of("bgm.wav"), Some("bgm.wav"));
    assert_eq!(file_name_of("/Users/a/Exhibit/"), None, "폴더 경로");
    assert_eq!(file_name_of(""), None);
}

#[test]
fn 저장된_경로에_있으면_그대로_두고_없으면_프로젝트_폴더에서_이름으로_찾는다() {
    let project = fresh_dir("basic");
    let here = touch(&project, "here.wav");
    let nested = touch(&project, "audio/room1/BGM.WAV");
    let windows = touch(&project, "sfx/Door.wav");
    let mut config = config_with(&[
        ("t1", &s(&here)),
        ("t2", "/Users/designer/Exhibit/audio/room1/bgm.wav"), // 대소문자가 다르다
        ("t3", r"C:\Shows\Exhibit\sfx\door.wav"),               // Windows에서 만든 경로
    ]);

    let report = relink_tracks(&mut config, std::slice::from_ref(&project));
    assert!(report.missing.is_empty(), "{:?}", report.missing);
    assert_eq!(paths(&config), vec![s(&here), s(&nested), s(&windows)]);
    assert_eq!(report.relinked.len(), 2, "그대로 있던 경로는 다시 연결하지 않는다: {:?}", report.relinked);
    assert_eq!(report.relinked[0], ("/Users/designer/Exhibit/audio/room1/bgm.wav".to_string(), s(&nested)));
    let _ = std::fs::remove_dir_all(project);
}

#[test]
fn 같은_이름이_여럿이면_상위_폴더_이름이_가장_많이_겹치는_것을_고른다() {
    let project = fresh_dir("ambiguous");
    touch(&project, "audio/room1/loop.wav");
    let room2 = touch(&project, "audio/room2/loop.wav");
    let shallow = touch(&project, "intro.wav");
    touch(&project, "backup/old/intro.wav");
    let mut config = config_with(&[
        ("t1", "/Users/designer/Exhibit/audio/room2/loop.wav"),
        ("t2", "/Volumes/USB/elsewhere/intro.wav"), // 겹치는 폴더가 없으면 얕은 쪽
    ]);

    let report = relink_tracks(&mut config, std::slice::from_ref(&project));
    assert!(report.missing.is_empty());
    assert_eq!(paths(&config), vec![s(&room2), s(&shallow)]);
    let _ = std::fs::remove_dir_all(project);
}

#[test]
fn 못_찾은_파일은_원래_경로를_두고_한_번씩만_알린다() {
    let project = fresh_dir("missing");
    touch(&project, "._gone.wav"); // macOS가 USB에 남기는 파일. 이름이 달라 후보가 아니다
    touch(&project, ".Trashes/gone.wav"); // 숨김 폴더는 뒤지지 않는다
    let mut config = config_with(&[
        ("t1", "/Users/designer/Exhibit/gone.wav"),
        ("t2", "/Users/designer/Exhibit/gone.wav"), // 두 트랙이 같은 파일을 쓴다
        ("t3", ""),                                 // 경로가 비어 있으면 건드리지 않는다
    ]);

    let report = relink_tracks(&mut config, std::slice::from_ref(&project));
    assert_eq!(report.missing, vec!["/Users/designer/Exhibit/gone.wav".to_string()]);
    assert!(report.relinked.is_empty());
    assert_eq!(
        paths(&config),
        vec!["/Users/designer/Exhibit/gone.wav".to_string(), "/Users/designer/Exhibit/gone.wav".to_string(), String::new()]
    );
    let _ = std::fs::remove_dir_all(project);
}

#[test]
fn 트랙_경로_말고_다른_설정_값은_그대로다() {
    let project = fresh_dir("untouched");
    let found = touch(&project, "a.wav");
    let mut config = config_with(&[("t1", "/Users/designer/a.wav")]);
    let mut expected = config_with(&[("t1", &s(&found))]);
    expected.rooms[0].tracks[0].file_path = s(&found);

    relink_tracks(&mut config, std::slice::from_ref(&project));
    assert_eq!(serde_json::to_value(&config).unwrap(), serde_json::to_value(&expected).unwrap());
    let _ = std::fs::remove_dir_all(project);
}

#[test]
fn 여러_폴더를_주면_앞의_폴더를_먼저_찾는다() {
    let project = fresh_dir("first");
    let chosen = fresh_dir("second");
    let in_project = touch(&project, "a.wav");
    touch(&chosen, "a.wav");
    let only_chosen = touch(&chosen, "deep/b.wav");
    let mut finder = MediaFinder::new(&[project.clone(), chosen.clone()]);
    assert_eq!(finder.resolve("/Users/designer/a.wav"), Some(in_project));
    assert_eq!(finder.resolve("/Users/designer/b.wav"), Some(only_chosen));
    assert_eq!(finder.resolve("/Users/designer/c.wav"), None);

    // 도면처럼 파일 하나만 찾을 때도 같다. 저장된 경로에 있으면 그대로다.
    let blueprint = touch(&project, "plan/floor.png");
    assert_eq!(MediaFinder::new(std::slice::from_ref(&chosen)).resolve(&s(&blueprint)), Some(blueprint.clone()));
    assert_eq!(MediaFinder::new(std::slice::from_ref(&project)).resolve(r"D:\plans\FLOOR.png"), Some(blueprint));
    for d in [project, chosen] {
        let _ = std::fs::remove_dir_all(d);
    }
}
