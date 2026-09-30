//! 스피커는 **자기가 속한 방**(Flutter SpeakerNode.roomId)으로 RoomZone에 연결되어야 한다.
//!
//! Flutter는 방마다 로컬 좌표(각 방의 원점 = 0,0)를 쓰기 때문에 모든 RoomZone이
//! 원점에서 겹쳐 전송된다. 예전에는 "스피커 좌표가 들어가는 첫 번째 방"으로 연결해서,
//! 방 2~5의 스피커도 좌표가 방 1 안에 들어가면 방 1의 초기반사·바이노럴 기준점을 썼다.
//! 방 ID가 없는 예전 payload는 좌표로 찾는 기존 동작을 유지한다.

use rust_lib_atmos_mixer_pro::api::simple::api_update_spatial_config_json;
use rust_lib_atmos_mixer_pro::audio::acoustic::{bind_channel_zone, compute_early_reflection_taps};
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::common::config::{Point3D, RoomZone};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;

const BIG: u32 = 111;
const SMALL: u32 = 222;

fn point(x: f32, y: f32, z: f32) -> Point3D {
    Point3D { x, y, z, ..Default::default() }
}

/// 둘 다 원점에서 시작한다(Flutter buildRoomZonesPayload와 같은 모양).
fn big_room() -> RoomZone {
    RoomZone {
        room_id: BIG,
        boundary_min: point(0.0, 0.0, 0.0),
        boundary_max: point(15.0, 20.0, 15.0),
        absorption_coeff: 0.02,
        ear_level: 1.6,
        ..Default::default()
    }
}

fn small_room() -> RoomZone {
    RoomZone {
        room_id: SMALL,
        boundary_min: point(0.0, 0.0, 0.0),
        boundary_max: point(5.0, 5.0, 3.0),
        absorption_coeff: 0.1,
        ear_level: 1.2,
        ..Default::default()
    }
}

#[test]
fn 스피커는_좌표가_아니라_자기_방_id로_방에_연결된다() {
    let zones = [big_room(), small_room()];
    let pos = point(1.0, 1.0, 2.0); // 두 방 모두에 들어가는 좌표

    assert_eq!(bind_channel_zone(&zones, Some(SMALL), &pos).map(|z| z.room_id), Some(SMALL));
    assert_eq!(bind_channel_zone(&zones, Some(BIG), &pos).map(|z| z.room_id), Some(BIG));
    // 방 ID가 없는 예전 payload: 좌표가 들어가는 첫 번째 방.
    assert_eq!(bind_channel_zone(&zones, None, &pos).map(|z| z.room_id), Some(BIG));
    // 지워진 방을 가리키면 다른 방에 붙이지 않는다.
    assert_eq!(bind_channel_zone(&zones, Some(999), &pos).map(|z| z.room_id), None);
}

#[test]
fn 초기반사_탭은_스피커가_속한_방으로_계산된다() {
    // Flutter가 보내는 JSON 그대로. 세 채널 모두 같은 좌표에 있다.
    let json = format!(
        r#"{{
        "channel_positions": [
            {{"x": 1.0, "y": 1.0, "z": 2.0, "room_id": {SMALL}}},
            {{"x": 1.0, "y": 1.0, "z": 2.0, "room_id": {BIG}}},
            {{"x": 1.0, "y": 1.0, "z": 2.0}}
        ],
        "room_zones": [
            {{"room_id": {BIG}, "boundary_min": {{"x":0,"y":0,"z":0}}, "boundary_max": {{"x":15,"y":20,"z":15}}, "absorption_coeff": 0.02, "ear_level": 1.6}},
            {{"room_id": {SMALL}, "boundary_min": {{"x":0,"y":0,"z":0}}, "boundary_max": {{"x":5,"y":5,"z":3}}, "absorption_coeff": 0.1, "ear_level": 1.2}}
        ],
        "trajectory": null,
        "track_positions": {{}}
    }}"#
    );
    while GLOBAL_STATE.command_receiver.try_recv().is_ok() {}
    api_update_spatial_config_json(json).expect("payload 처리 실패");

    let (taps, room_ids) = loop {
        match GLOBAL_STATE.command_receiver.try_recv() {
            Ok(AudioCommand::UpdateSpatialConfig { early_reflection_taps, channel_room_ids, .. }) => {
                break (early_reflection_taps, channel_room_ids)
            }
            Ok(_) => continue,
            Err(_) => panic!("UpdateSpatialConfig 명령이 오지 않았다"),
        }
    };

    assert_eq!(room_ids, vec![Some(SMALL), Some(BIG), None]);

    let pos = point(1.0, 1.0, 2.0);
    let expected_small = compute_early_reflection_taps(&pos, &small_room());
    let expected_big = compute_early_reflection_taps(&pos, &big_room());
    for i in 0..6 {
        assert_eq!(taps[0][i].delay_ms, expected_small[i].delay_ms, "ch0(작은 방) 탭 {i}");
        assert_eq!(taps[0][i].gain, expected_small[i].gain, "ch0(작은 방) 탭 {i}");
        assert_eq!(taps[1][i].delay_ms, expected_big[i].delay_ms, "ch1(큰 방) 탭 {i}");
        assert_eq!(taps[2][i].delay_ms, expected_big[i].delay_ms, "ch2(방 ID 없음) 탭 {i}");
    }
    // 같은 좌표라도 방이 다르면 반사가 달라야 한다.
    assert_ne!(taps[0][1].delay_ms, taps[1][1].delay_ms, "천장 반사가 방마다 같다");
}

#[test]
fn 보고_있는_방이_payload로_엔진까지_전달된다() {
    let json = format!(
        r#"{{
        "listener_position": {{"x": 2.5, "y": 2.5, "z": 1.2}},
        "active_room_id": {SMALL},
        "channel_positions": [{{"x": 1.0, "y": 1.0, "z": 2.0, "room_id": {SMALL}}}],
        "room_zones": [],
        "trajectory": null,
        "track_positions": {{}}
    }}"#
    );
    while GLOBAL_STATE.command_receiver.try_recv().is_ok() {}
    api_update_spatial_config_json(json).expect("payload 처리 실패");

    let active = loop {
        match GLOBAL_STATE.command_receiver.try_recv() {
            Ok(AudioCommand::UpdateSpatialConfig { active_room_id, .. }) => break active_room_id,
            Ok(_) => continue,
            Err(_) => panic!("UpdateSpatialConfig 명령이 오지 않았다"),
        }
    };
    assert_eq!(active, Some(SMALL), "보고 있는 방이 엔진에 전달되지 않았다");
}

#[test]
fn 바이노럴_기준점도_스피커가_속한_방의_중심이다() {
    let azimuth_with = |zones: Vec<RoomZone>, room_ids: Vec<Option<u32>>| {
        let (gc_tx, _) = crossbeam_channel::unbounded();
        let mut mixer = AudioMixer::new(48000, 2, 512, gc_tx, None);
        mixer.listener_position = None; // 방 중심을 기준점으로 쓰는 경로
        mixer.channel_positions = vec![Some(point(1.0, 1.0, 1.5))];
        mixer.room_zones = zones;
        mixer.channel_room_ids = room_ids;
        mixer.recalculate_binaural_channel_azimuths();
        mixer.binaural.channel_base_azimuth_mut()[0]
    };

    let in_small = azimuth_with(vec![big_room(), small_room()], vec![Some(SMALL)]);
    let small_only = azimuth_with(vec![small_room()], vec![None]);
    let big_only = azimuth_with(vec![big_room()], vec![None]);

    assert_eq!(in_small, small_only, "작은 방 스피커가 작은 방 중심 기준이 아니다");
    assert_ne!(in_small, big_only, "두 방의 중심이 같은 방위를 내서 검증이 무의미하다");
}
