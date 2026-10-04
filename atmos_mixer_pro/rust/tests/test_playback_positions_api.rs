//! 재생 위치 조회는 재생 목록에 있는 인스턴스의 위치만 트랙 이름으로 돌려준다(엔진이 버린 옛 칸은 무시).
//! 전역 상태를 쓰므로 테스트는 하나다.
use rust_lib_atmos_mixer_pro::api::simple::api_get_playback_positions;
use rust_lib_atmos_mixer_pro::audio::playback_cursor::CURSOR_TABLE;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;

#[test]
fn 재생_목록에_있는_인스턴스만_트랙별_위치로_돌려준다() {
    GLOBAL_STATE.clear_playing_tracks();
    CURSOR_TABLE.clear_all();
    GLOBAL_STATE.add_playing_track(1, "a".into());
    GLOBAL_STATE.add_playing_track(2, "b".into());
    CURSOR_TABLE.publish(0, 1, 4.5);
    CURSOR_TABLE.publish(1, 2, 0.75);
    CURSOR_TABLE.publish(2, 3, 9.0); // 재생 목록에 없는 옛 칸
    let got: Vec<(String, f64)> = api_get_playback_positions()
        .into_iter()
        .map(|p| (p.track_id, p.seconds))
        .collect();
    assert_eq!(got, vec![("a".to_string(), 4.5), ("b".to_string(), 0.75)]);
}
