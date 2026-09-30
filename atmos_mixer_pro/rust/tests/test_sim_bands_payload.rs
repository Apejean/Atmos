//! 헤드폰 미리듣기의 현장 물리 밴드는 공간 설정 payload의 채널 항목(`sim_bands`)으로 실려 온다.
//! Flutter가 보내는 JSON 그대로 넣어서 엔진 명령에 슬롯 순서대로 실리는지 본다.
//! 같은 파일의 테스트는 GLOBAL_STATE 명령 큐를 공유하므로 이 파일에는 테스트를 하나만 둔다.

use rust_lib_atmos_mixer_pro::api::simple::api_update_spatial_config_json;
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::common::config::EqType;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;

#[test]
fn 스피커별_현장_물리_밴드가_슬롯_순서대로_엔진까지_전달된다() {
    let json = r#"{
        "channel_positions": [
            {"x": 1.0, "y": 1.0, "z": 2.0, "room_id": 11, "sim_bands": [
                {"on": true, "type": 1, "freq": 245.0, "gain": 3.5, "q": 0.707},
                {"on": false, "type": 2, "freq": 60.0, "gain": 0.0, "q": 4.0},
                {"on": true, "type": 2, "freq": 68.6, "gain": 2.8, "q": 6.2}
            ]},
            {"x": 2.0, "y": 1.0, "z": 2.0, "room_id": 11},
            null
        ],
        "room_zones": [],
        "trajectory": null,
        "track_positions": {}
    }"#;
    while GLOBAL_STATE.command_receiver.try_recv().is_ok() {}
    api_update_spatial_config_json(json.to_string()).expect("payload 처리 실패");
    let bands = loop {
        match GLOBAL_STATE.command_receiver.try_recv() {
            Ok(AudioCommand::UpdateSpatialConfig { channel_sim_bands, .. }) => break channel_sim_bands,
            Ok(_) => continue,
            Err(_) => panic!("UpdateSpatialConfig 명령이 오지 않았다"),
        }
    };
    assert_eq!(bands.len(), 3);
    let b = &bands[0];
    assert!(b[0].enabled && b[0].filter_type == EqType::LowShelf);
    assert_eq!((b[0].freq, b[0].gain, b[0].q_factor), (245.0, 3.5, 0.707));
    assert!(!b[1].enabled, "꺼진 슬롯은 꺼진 채로 와야 한다");
    assert!(b[2].enabled && b[2].filter_type == EqType::Bell);
    assert_eq!((b[2].freq, b[2].gain, b[2].q_factor), (68.6, 2.8, 6.2));
    assert!(b[3..].iter().all(|x| !x.enabled), "보내지 않은 슬롯은 꺼져 있어야 한다");
    assert!(bands[1].iter().all(|x| !x.enabled), "물리 밴드가 없는 채널은 전부 꺼짐");
    assert!(bands[2].iter().all(|x| !x.enabled), "스피커 없는 채널은 전부 꺼짐");
}
