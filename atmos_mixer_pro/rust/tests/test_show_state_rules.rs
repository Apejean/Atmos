//! 공연 이어 가기 판단, 저장 형식, 무음 판단(core::show_state). 전역 상태를 쓰지 않는다.
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::show_state::{
    decide_resume, load, save, silence_condition, ResumePlan, SavedTrack, ShowState, SilenceWatch,
    MAX_RESUME_AGE_MS, SHOW_STATE_FILE,
};

const NOW: u64 = 1_759_650_000_000;

fn track(id: &str) -> TrackConfig {
    TrackConfig {
        id: id.into(),
        name: id.into(),
        file_path: format!("/{id}.wav"),
        volume: 1.0,
        is_loop: false,
        is_streaming: false,
        output_channel: 0,
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
        volume_osc_address: String::new(),
        clear_osc_address: String::new(),
        tracks,
    }
}

fn config() -> AppConfig {
    AppConfig {
        rooms: vec![room("a", vec![track("la"), track("oa")]), room("b", vec![track("lb")])],
        ..AppConfig::default()
    }
}

fn saved(room: &str, track: &str, seconds: f64) -> SavedTrack {
    SavedTrack { room_id: room.into(), track_id: track.into(), seconds }
}

#[test]
fn 이어_가기는_10분_안에_저장했고_지금_설정에_남은_트랙이_있을_때만이다() {
    let cfg = config();
    let s = ShowState {
        saved_at_ms: NOW - 5_000,
        active_room_id: Some("a".into()),
        tracks: vec![saved("a", "la", 12.5), saved("a", "oa", 3.0)],
    };
    assert_eq!(
        decide_resume(Some(&s), NOW, Some(&cfg)),
        ResumePlan::Resume { active_room_id: Some("a".into()), tracks: s.tracks.clone() }
    );

    // 지금 설정에서 사라진 트랙(다른 방으로 옮겨진 것 포함)은 빼고 이어 간다
    let moved = ShowState {
        tracks: vec![saved("a", "la", 12.5), saved("a", "gone", 1.0), saved("x", "lb", 2.0)],
        ..s.clone()
    };
    assert_eq!(
        decide_resume(Some(&moved), NOW, Some(&cfg)),
        ResumePlan::Resume { active_room_id: Some("a".into()), tracks: vec![saved("a", "la", 12.5)] }
    );

    // 활성 방이 설정에서 사라졌으면 활성 방 없이 이어 간다
    let no_room = ShowState { active_room_id: Some("gone".into()), ..s.clone() };
    assert!(matches!(
        decide_resume(Some(&no_room), NOW, Some(&cfg)),
        ResumePlan::Resume { active_room_id: None, .. }
    ));

    // 딱 10분은 이어 간다
    let edge = ShowState { saved_at_ms: NOW - MAX_RESUME_AGE_MS, ..s.clone() };
    assert!(matches!(decide_resume(Some(&edge), NOW, Some(&cfg)), ResumePlan::Resume { .. }));

    // 첫 방 테마로 가는 경우
    let theme = |plan: ResumePlan| matches!(plan, ResumePlan::ThemeStart(_));
    assert!(theme(decide_resume(None, NOW, Some(&cfg))), "저장 파일 없음");
    let old = ShowState { saved_at_ms: NOW - MAX_RESUME_AGE_MS - 1, ..s.clone() };
    assert!(theme(decide_resume(Some(&old), NOW, Some(&cfg))), "10분 지남");
    let gone = ShowState { tracks: vec![saved("a", "gone", 1.0)], ..s.clone() };
    assert!(theme(decide_resume(Some(&gone), NOW, Some(&cfg))), "트랙이 설정에서 사라짐");
    let idle = ShowState { tracks: vec![], ..s.clone() };
    assert!(theme(decide_resume(Some(&idle), NOW, Some(&cfg))), "재생 중이던 게 없었음");
    assert!(theme(decide_resume(Some(&s), NOW, None)), "설정이 없음");
    let future = ShowState { saved_at_ms: NOW + 3_600_000, ..s.clone() };
    assert!(theme(decide_resume(Some(&future), NOW, Some(&cfg))), "저장 시각이 한 시간 뒤(시계가 뒤로 감)");
}

#[test]
fn 저장_파일은_고스란히_다시_읽히고_깨진_파일은_오류다() {
    let dir = std::env::temp_dir().join(format!("atmos_show_rules_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    assert!(load(&dir).unwrap().is_none(), "파일이 없으면 None");

    let s = ShowState {
        saved_at_ms: NOW,
        active_room_id: Some("a".into()),
        tracks: vec![saved("a", "la", 12.5), saved("b", "lb", 0.25)],
    };
    save(&dir, &s).unwrap();
    assert_eq!(load(&dir).unwrap(), Some(s));
    assert!(!dir.join(format!("{SHOW_STATE_FILE}.tmp")).exists(), "임시 파일이 남았다");

    std::fs::write(dir.join(SHOW_STATE_FILE), "{\"saved_at_ms\": 1, \"trac").unwrap();
    assert!(load(&dir).is_err(), "깨진 파일을 읽었다고 했다");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 무음은_60초가_지나면_경고하고_이어지면_10분마다_다시_경고한다() {
    let mut w = SilenceWatch::default();
    // 저장 스레드처럼 5초마다 본다
    let warned: Vec<u64> = (0..=150u64).map(|i| i * 5_000).filter(|&t| w.observe(t, true)).collect();
    assert_eq!(warned, vec![60_000, 660_000]);
    assert!(!w.observe(755_000, false), "소리가 나면 경고하지 않는다");
    assert!(!w.observe(760_000, true), "다시 처음부터 잰다");
    assert!(!w.observe(815_000, true));
    assert!(w.observe(820_000, true));
}

#[test]
fn 무음_조건은_루프_재생_중이고_all_mute가_꺼져_있고_모든_피크가_0일_때다() {
    assert!(silence_condition(true, false, [0.0, 0.0]));
    assert!(!silence_condition(false, false, [0.0, 0.0]), "루프가 안 돈다(단발 사이 정적은 정상)");
    assert!(!silence_condition(true, true, [0.0, 0.0]), "All Mute");
    assert!(!silence_condition(true, false, [0.0, 1e-6]), "아주 작은 소리라도 난다");
    assert!(silence_condition(true, false, []), "출력 채널이 없으면 나가는 소리도 없다");
}
