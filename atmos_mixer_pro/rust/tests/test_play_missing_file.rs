//! 파일이 없는 트랙을 재생하면 이유를 분명히 돌려주고 앱 로그에 남긴다.
//!
//! 2026-10-10 Windows P6-E: 맥에서 저장한 프로젝트를 열었는데 파일 하나를 끝까지 못 찾았다. 그 트랙을
//! 재생하자 로딩 단계의 영어 오류만 돌아가 화면에는 "치명적 시스템 오류"로만 보였고 앱 로그에는 아무것도
//! 없었다. 전역 상태를 쓰므로 테스트는 하나다.
use rust_lib_atmos_mixer_pro::api::simple::api_play_track;
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;

fn track(id: &str, path: &str, is_loop: bool) -> TrackConfig {
    TrackConfig {
        id: id.into(),
        name: format!("{id} 이름"),
        file_path: path.into(),
        volume: 1.0,
        is_loop,
        is_streaming: false,
        output_channel: 0,
        output_stereo: false,
        play_osc_address: String::new(),
        stop_osc_address: String::new(),
    }
}

#[test]
fn 파일이_없는_트랙을_재생하면_경로가_든_오류를_돌려주고_재생_목록에_넣지_않는다() {
    let missing_sfx = "/Users/Allweno/Downloads/없는-효과음.wav";
    let missing_bgm = "/Users/Allweno/Downloads/없는-배경음.wav";
    let mut config = AppConfig::default();
    config.rooms.push(RoomConfig {
        id: "room_missing".into(),
        name: "테마 2".into(),
        color_hex: "#ffffff".into(),
        volume: 1.0,
        volume_osc_address: String::new(),
        clear_osc_address: String::new(),
        tracks: vec![track("sfx", missing_sfx, false), track("bgm", missing_bgm, true)],
    });
    *GLOBAL_STATE.config.write().unwrap() = Some(config);
    GLOBAL_STATE.clear_playing_tracks();

    // 단발(메모리에 읽는 경로)과 루프(스트리밍 경로) 모두 같은 이유를 돌려준다.
    for (id, path) in [("sfx", missing_sfx), ("bgm", missing_bgm)] {
        let err = api_play_track("room_missing".into(), id.into())
            .expect_err("없는 파일인데 재생을 받아들였다");
        assert_eq!(err.message, format!("파일을 찾을 수 없습니다: {path}"));
    }
    assert!(
        GLOBAL_STATE.playing_track_ids.read().unwrap().is_empty(),
        "재생하지 못한 트랙이 재생 목록에 남았다"
    );
}
