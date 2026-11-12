//! 테마 시작은 루프 트랙을 한 그룹으로 대기시켜 보낸 뒤, 마지막에 StartGroup 하나를 보낸다(그룹 시작 설계 3.1).
//! 그래야 보내는 도중 콜백 경계가 지나가도 모든 루프가 같은 샘플에 시작한다(진단 T252: 지금은 1블록씩 갈렸다).
//! 전역 상태(명령 큐·설정)를 쓰므로 이 파일의 시험은 하나다.
use rust_lib_atmos_mixer_pro::api::show::api_theme_start;
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
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

fn drain() -> Vec<AudioCommand> {
    let mut v = Vec::new();
    while let Ok(c) = GLOBAL_STATE.command_receiver.try_recv() {
        v.push(c);
    }
    v
}

#[test]
fn 테마_시작은_루프를_한_그룹으로_대기시키고_startgroup을_마지막에_보낸다() {
    let dir = std::env::temp_dir().join(format!("atmos_group_theme_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("시험 폴더");
    let p = |n: &str| dir.join(format!("{n}.wav")).to_string_lossy().into_owned();
    for name in ["l1", "l2", "l3"] {
        tone_wav(&dir.join(format!("{name}.wav")), 2.0);
    }
    GLOBAL_STATE.engine_sample_rate.store(48_000, Ordering::SeqCst);
    *GLOBAL_STATE.config.write().expect("설정") = Some(AppConfig {
        rooms: vec![RoomConfig {
            id: "a".into(),
            name: "a".into(),
            color_hex: "#ffffff".into(),
            volume: 1.0,
            clear_osc_address: String::new(),
            tracks: vec![
                track("l1", &p("l1"), true, 0),
                track("l2", &p("l2"), true, 1),
                track("l3", &p("l3"), true, 2),
            ],
        }],
        ..AppConfig::default()
    });
    GLOBAL_STATE.clear_playing_tracks();
    drain();

    api_theme_start().expect("테마 시작");
    let cmds = drain();
    let plays: Vec<(usize, u64, bool)> = cmds
        .iter()
        .enumerate()
        .filter_map(|(i, c)| match c {
            AudioCommand::PlayTrack { instance, .. } => Some((i, instance.group_id, instance.armed)),
            _ => None,
        })
        .collect();
    assert_eq!(plays.len(), 3, "루프 셋을 모두 보내야 한다");
    let group = plays[0].1;
    assert_ne!(group, 0, "테마 시작의 루프는 그룹이어야 한다");
    assert!(plays.iter().all(|&(_, g, armed)| g == group && armed), "모든 루프가 같은 그룹의 대기여야 한다: {plays:?}");
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
    assert!(
        !cmds.iter().any(|c| matches!(c, AudioCommand::CancelGroup { .. })),
        "정지가 없었는데 그룹을 취소했다"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
