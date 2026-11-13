//! 대시보드 클리어 버튼(api_clear_room_and_advance)은 OSC 방 클리어와 같은 공통 함수로 이 방을 비우고, 다음 방을
//! 활성으로 올려 그 방의 루프를 한 그룹으로 같은 샘플에 함께 튼다(그룹 시작 설계). 예전에는 화면(Dart)이 다음 방
//! 루프를 하나씩 틀어 트랙끼리 어긋날 수 있었다. 전역 상태(명령 큐·설정·활성 방)를 쓰므로 이 파일의 시험은 하나다.
use rust_lib_atmos_mixer_pro::api::simple::api_clear_room_and_advance;
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::common::utils::hash_id;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;

fn tone_wav(path: &std::path::Path, seconds: f32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 48_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).expect("시험 파일");
    for i in 0..(48_000.0 * seconds) as usize {
        let v = (i as f32 * 200.0 * std::f32::consts::TAU / 48_000.0).sin() * 3000.0;
        w.write_sample(v as i16).expect("시험 파일 쓰기");
    }
    w.finalize().expect("시험 파일 닫기");
}

fn track(id: &str, path: &str, is_loop: bool, out: u32) -> TrackConfig {
    TrackConfig {
        id: id.into(),
        name: id.into(),
        file_path: path.into(),
        volume: 1.0,
        is_loop,
        is_streaming: false,
        output_channel: out,
        output_stereo: false,
        play_osc_address: String::new(),
        stop_osc_address: String::new(),
    }
}

fn room(id: &str, tracks: Vec<TrackConfig>) -> RoomConfig {
    RoomConfig {
        id: id.into(),
        name: id.into(),
        color_hex: "#ffffff".into(),
        volume: 1.0,
        clear_osc_address: String::new(),
        tracks,
    }
}

fn drain() -> Vec<AudioCommand> {
    let mut v = Vec::new();
    while let Ok(c) = GLOBAL_STATE.command_receiver.try_recv() {
        v.push(c);
    }
    v
}

fn active_room() -> Option<String> {
    GLOBAL_STATE.active_room_id.read().expect("활성 방").clone()
}

fn set_active(room_id: Option<&str>) {
    GLOBAL_STATE.set_active_room(room_id.map(str::to_string));
}

/// 명령 가운데 ClearRoom의 방(해시) 목록.
fn cleared(cmds: &[AudioCommand]) -> Vec<u32> {
    cmds.iter()
        .filter_map(|c| match c {
            AudioCommand::ClearRoom { room_id } => Some(*room_id),
            _ => None,
        })
        .collect()
}

#[test]
fn 대시보드_클리어는_다음_방_루프를_한_그룹으로_틀고_두_번째_누름은_막는다() {
    let dir = std::env::temp_dir().join(format!("atmos_clear_advance_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("시험 폴더");
    let p = |n: &str| dir.join(format!("{n}.wav")).to_string_lossy().into_owned();
    for name in ["la", "b1", "b2", "b3", "bs"] {
        tone_wav(&dir.join(format!("{name}.wav")), 2.0);
    }
    GLOBAL_STATE.engine_sample_rate.store(48_000, Ordering::SeqCst);
    let config = AppConfig {
        rooms: vec![
            room("a", vec![track("la", &p("la"), true, 0)]),
            // 한 작품을 채널별로 나눈 모노 루프 셋과, 신호로만 트는 효과음 하나
            room(
                "b",
                vec![
                    track("b1", &p("b1"), true, 0),
                    track("b2", &p("b2"), true, 1),
                    track("b3", &p("b3"), true, 2),
                    track("bs", &p("bs"), false, 3),
                ],
            ),
            room("c", vec![]),
        ],
        is_exhibition_mode: false,
        ..AppConfig::default()
    };
    *GLOBAL_STATE.config.write().expect("설정") = Some(config.clone());
    GLOBAL_STATE.clear_playing_tracks();
    set_active(Some("a"));
    drain();

    // 1) a를 클리어하면 a를 비우고 b가 활성이 되며, b의 루프 셋만 한 그룹으로 대기했다가 StartGroup 하나로 함께 시작한다.
    let result = api_clear_room_and_advance("a".into()).expect("a 클리어");
    assert_eq!(result.next_room_id.as_deref(), Some("b"));
    assert!(result.failed_tracks.is_empty(), "다음 방 루프가 실패했다: {:?}", result.failed_tracks);
    assert_eq!(active_room().as_deref(), Some("b"));
    let cmds = drain();
    assert_eq!(cleared(&cmds), vec![hash_id("a")], "a만 비워야 한다");
    let clear_at = cmds.iter().position(|c| matches!(c, AudioCommand::ClearRoom { .. })).unwrap_or(usize::MAX);
    let plays: Vec<(usize, u32, u64, bool)> = cmds
        .iter()
        .enumerate()
        .filter_map(|(i, c)| match c {
            AudioCommand::PlayTrack { instance, .. } => Some((i, instance.room_id, instance.group_id, instance.armed)),
            _ => None,
        })
        .collect();
    assert_eq!(plays.len(), 3, "b의 루프 셋만 틀어야 한다(효과음은 신호로만): {plays:?}");
    let group = plays[0].2;
    assert_ne!(group, 0, "다음 방 루프는 그룹이어야 한다");
    assert!(
        plays.iter().all(|&(_, room, g, armed)| room == hash_id("b") && g == group && armed),
        "모든 루프가 b 방의 같은 그룹 대기여야 한다: {plays:?}"
    );
    assert!(plays.iter().all(|p| p.0 > clear_at), "비우기보다 다음 방 재생이 먼저 나갔다");
    let starts: Vec<usize> = cmds
        .iter()
        .enumerate()
        .filter_map(|(i, c)| match c {
            AudioCommand::StartGroup { group_id } if *group_id == group => Some(i),
            _ => None,
        })
        .collect();
    assert_eq!(starts.len(), 1, "StartGroup은 하나여야 한다");
    assert!(starts[0] > plays.iter().map(|p| p.0).max().unwrap_or(0), "StartGroup이 재생 명령보다 먼저 나갔다");
    assert!(!cmds.iter().any(|c| matches!(c, AudioCommand::CancelGroup { .. })), "정지가 없었는데 그룹을 취소했다");

    // 2) 버튼을 한 번 더 눌러도(a는 이미 활성이 아님) 아무것도 하지 않는다. 다음 방까지 비우면 안 된다.
    assert!(api_clear_room_and_advance("a".into()).is_err(), "두 번째 누름이 받아들여졌다");
    assert!(drain().is_empty(), "두 번째 누름이 명령을 보냈다");
    assert_eq!(active_room().as_deref(), Some("b"));

    // 3) 루프가 없는 다음 방으로 넘기면 비우기만 하고 그룹은 만들지 않는다.
    let result = api_clear_room_and_advance("b".into()).expect("b 클리어");
    assert_eq!(result.next_room_id.as_deref(), Some("c"));
    let cmds = drain();
    assert_eq!(cleared(&cmds), vec![hash_id("b")]);
    assert!(
        !cmds
            .iter()
            .any(|c| matches!(c, AudioCommand::PlayTrack { .. } | AudioCommand::StartGroup { .. })),
        "루프가 없는 방인데 재생했다"
    );
    assert_eq!(active_room().as_deref(), Some("c"));

    // 4) 마지막 방을 비우면 활성 방이 없어진다.
    let result = api_clear_room_and_advance("c".into()).expect("c 클리어");
    assert_eq!(result.next_room_id, None);
    assert_eq!(cleared(&drain()), vec![hash_id("c")]);
    assert_eq!(active_room(), None);

    // 5) 전시 모드에서는 비우지 않는다(OSC 클리어와 같음).
    *GLOBAL_STATE.config.write().expect("설정") = Some(AppConfig {
        is_exhibition_mode: true,
        ..config
    });
    set_active(Some("a"));
    drain();
    assert!(api_clear_room_and_advance("a".into()).is_err(), "전시 모드에서 클리어했다");
    assert!(drain().is_empty(), "전시 모드에서 명령을 보냈다");
    assert_eq!(active_room().as_deref(), Some("a"));

    let _ = std::fs::remove_dir_all(&dir);
}
