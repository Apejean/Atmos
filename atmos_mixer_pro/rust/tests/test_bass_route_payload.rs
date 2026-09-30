//! 서브우퍼 지정은 스피커 속성(`is_subwoofer`)으로 공간 설정 payload에 실려 온다.
//!
//! Flutter가 보내는 JSON 그대로 넣어서, 엔진 명령(UpdateSpatialConfig)에 방별 라우팅 표가
//! 실리는지 본다. 같은 파일의 테스트는 GLOBAL_STATE 명령 큐를 공유하므로 이 파일에는
//! 테스트를 하나만 둔다.

use rust_lib_atmos_mixer_pro::api::simple::api_update_spatial_config_json;
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;

fn send_and_receive(json: &str) -> (Vec<bool>, Vec<Option<usize>>) {
    while GLOBAL_STATE.command_receiver.try_recv().is_ok() {}
    api_update_spatial_config_json(json.to_string()).expect("payload 처리 실패");
    loop {
        match GLOBAL_STATE.command_receiver.try_recv() {
            Ok(AudioCommand::UpdateSpatialConfig { channel_is_sub, bass_route, .. }) => {
                break (channel_is_sub, bass_route)
            }
            Ok(_) => continue,
            Err(_) => panic!("UpdateSpatialConfig 명령이 오지 않았다"),
        }
    }
}

#[test]
fn 스피커의_서브_지정이_방별_라우팅_표로_엔진까지_전달된다() {
    // 화면 기준 CH1·CH2 = 방 A 메인, CH3 = 방 A 서브, CH4 = 방 B 메인(방 B엔 서브 없음),
    // CH5 = 스피커 없음(배열 인덱스는 0부터).
    let json = r#"{
        "channel_positions": [
            {"x": 1.0, "y": 1.0, "z": 2.0, "room_id": 11},
            {"x": 2.0, "y": 1.0, "z": 2.0, "room_id": 11},
            {"x": 3.0, "y": 1.0, "z": 0.3, "room_id": 11, "is_subwoofer": true},
            {"x": 1.0, "y": 1.0, "z": 2.0, "room_id": 22},
            null
        ],
        "room_zones": [],
        "trajectory": null,
        "track_positions": {}
    }"#;
    let (is_sub, route) = send_and_receive(json);
    assert_eq!(is_sub, vec![false, false, true, false, false]);
    assert_eq!(
        route,
        vec![Some(2), Some(2), None, None, None],
        "방 A 메인은 CH3 서브로, 서브가 없는 방 B와 빈 채널은 풀레인지여야 한다"
    );

    // is_subwoofer가 없는 예전 payload: 서브 없음 → 모두 풀레인지.
    let legacy = r#"{
        "channel_positions": [
            {"x": 1.0, "y": 1.0, "z": 2.0, "room_id": 11},
            {"x": 3.0, "y": 1.0, "z": 0.3, "room_id": 11}
        ],
        "room_zones": [],
        "trajectory": null,
        "track_positions": {}
    }"#;
    let (is_sub, route) = send_and_receive(legacy);
    assert_eq!(is_sub, vec![false, false]);
    assert_eq!(route, vec![None, None]);
}
