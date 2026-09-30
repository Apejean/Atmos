//! 현장형 다중 방 시나리오: 방 3개(대형 홀 6채널, 중형 4채널, 소형 2채널)가 모두
//! 원점 기준 로컬 좌표라 엔진 안에서 겹친다. 중형·소형 방 스피커는 좌표상 대형 홀 안에도
//! 들어간다. 각 채널이 **자기 방**으로 계산되는지, 방 연결을 쓰는 네 경로 모두에서 확인한다.
//!
//! 판정 방식: 여러 방이 겹친 믹서의 채널 결과가 "그 방과 그 방 스피커만 있는 믹서"의
//! 결과와 같아야 한다. 계산식 자체를 테스트에 다시 쓰지 않고 연결만 검증한다.

use rust_lib_atmos_mixer_pro::api::simple::api_update_spatial_config_json;
use rust_lib_atmos_mixer_pro::audio::acoustic::compute_early_reflection_taps;
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::common::config::{Point3D, RoomZone, Trajectory};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;

const CH: usize = 16;

fn p(x: f32, y: f32, z: f32) -> Point3D {
    Point3D { x, y, z, ..Default::default() }
}

fn zone(room_id: u32, w: f32, d: f32, h: f32, absorption: f32, ear: f32) -> RoomZone {
    RoomZone {
        room_id,
        boundary_min: p(0.0, 0.0, 0.0),
        boundary_max: p(w, d, h),
        absorption_coeff: absorption,
        ear_level: ear,
        ..Default::default()
    }
}

fn rooms() -> Vec<RoomZone> {
    vec![
        zone(1001, 20.0, 15.0, 8.0, 0.02, 1.6), // 대형 홀 A
        zone(1002, 8.0, 6.0, 4.0, 0.1, 1.2),    // 중형 B
        zone(1003, 5.0, 5.0, 3.0, 0.3, 1.2),    // 소형 C
    ]
}

/// (채널, 방 ID, 로컬 좌표)
fn speakers() -> Vec<(usize, u32, Point3D)> {
    vec![
        (0, 1001, p(2.0, 2.0, 6.0)),
        (1, 1001, p(18.0, 2.0, 6.0)),
        (2, 1001, p(2.0, 13.0, 6.0)),
        (3, 1001, p(18.0, 13.0, 6.0)),
        (4, 1001, p(10.0, 1.0, 7.0)),
        (5, 1001, p(10.0, 14.0, 7.0)),
        (6, 1002, p(1.0, 1.0, 2.5)),
        (7, 1002, p(7.0, 1.0, 2.5)),
        (8, 1002, p(1.0, 5.0, 2.5)),
        (9, 1002, p(7.0, 5.0, 2.5)),
        (10, 1003, p(1.0, 1.0, 2.2)),
        (11, 1003, p(4.0, 4.0, 2.2)),
    ]
}

fn zone_of(room_id: u32) -> RoomZone {
    rooms().into_iter().find(|z| z.room_id == room_id).unwrap()
}

/// 모든 방·스피커가 겹친 현장 믹서.
fn field_mixer() -> AudioMixer {
    let (gc_tx, _) = crossbeam_channel::unbounded();
    let mut m = AudioMixer::new(48000, CH, 512, gc_tx, None);
    m.listener_position = None;
    m.room_zones = rooms();
    m.channel_positions = vec![None; CH];
    m.channel_room_ids = vec![None; CH];
    for (ch, rid, pos) in speakers() {
        m.channel_positions[ch] = Some(pos);
        m.channel_room_ids[ch] = Some(rid);
    }
    m
}

/// 방 하나와 그 방 스피커만 있는 믹서(방 ID 없이 좌표로 연결 = 예전 방식이 확실히 맞는 경우).
fn isolated_mixer(room_id: u32) -> AudioMixer {
    let (gc_tx, _) = crossbeam_channel::unbounded();
    let mut m = AudioMixer::new(48000, CH, 512, gc_tx, None);
    m.listener_position = None;
    m.room_zones = vec![zone_of(room_id)];
    m.channel_positions = vec![None; CH];
    for (ch, rid, pos) in speakers() {
        if rid == room_id {
            m.channel_positions[ch] = Some(pos);
        }
    }
    m
}

#[test]
fn 초기반사_모든_채널이_자기_방_형상으로_계산된다() {
    let channels: Vec<String> = (0..CH)
        .map(|ch| match speakers().into_iter().find(|s| s.0 == ch) {
            Some((_, rid, pos)) => format!(
                r#"{{"x":{},"y":{},"z":{},"room_id":{rid}}}"#,
                pos.x, pos.y, pos.z
            ),
            None => "null".to_string(),
        })
        .collect();
    let zones: Vec<String> = rooms()
        .iter()
        .map(|z| {
            format!(
                r#"{{"room_id":{},"boundary_min":{{"x":0,"y":0,"z":0}},"boundary_max":{{"x":{},"y":{},"z":{}}},"absorption_coeff":{},"ear_level":{}}}"#,
                z.room_id, z.boundary_max.x, z.boundary_max.y, z.boundary_max.z, z.absorption_coeff, z.ear_level
            )
        })
        .collect();
    let json = format!(
        r#"{{"channel_positions":[{}],"room_zones":[{}],"trajectory":null,"track_positions":{{}}}}"#,
        channels.join(","),
        zones.join(",")
    );

    while GLOBAL_STATE.command_receiver.try_recv().is_ok() {}
    api_update_spatial_config_json(json).expect("payload 처리 실패");
    let taps = loop {
        match GLOBAL_STATE.command_receiver.try_recv() {
            Ok(AudioCommand::UpdateSpatialConfig { early_reflection_taps, .. }) => break early_reflection_taps,
            Ok(_) => continue,
            Err(_) => panic!("UpdateSpatialConfig 명령이 오지 않았다"),
        }
    };

    for (ch, rid, pos) in speakers() {
        let expected = compute_early_reflection_taps(&pos, &zone_of(rid));
        for i in 0..6 {
            assert_eq!(taps[ch][i].delay_ms, expected[i].delay_ms, "ch{ch}(방 {rid}) 탭 {i} 지연");
            assert_eq!(taps[ch][i].gain, expected[i].gain, "ch{ch}(방 {rid}) 탭 {i} 크기");
        }
    }
    for (ch, ch_taps) in taps.iter().enumerate().take(CH).skip(12) {
        assert!(ch_taps.iter().all(|t| t.gain == 0.0), "스피커 없는 ch{ch}에 반사가 생겼다");
    }
}

#[test]
fn 바이노럴_기준점_모든_채널이_자기_방_중심을_쓴다() {
    let mut field = field_mixer();
    field.recalculate_binaural_channel_azimuths();
    let field_az = field.binaural.channel_base_azimuth_mut().to_vec();

    for rid in [1001, 1002, 1003] {
        let mut iso = isolated_mixer(rid);
        iso.recalculate_binaural_channel_azimuths();
        let iso_az = iso.binaural.channel_base_azimuth_mut().to_vec();
        for (ch, r, _) in speakers() {
            if r == rid {
                assert_eq!(field_az[ch], iso_az[ch], "ch{ch}(방 {rid}) 방위각");
            }
        }
    }
}

#[test]
fn 헤드폰_전파_거리도_자기_방_기준이다() {
    // 공기 흡음·전파 지연·거리 감쇠는 헤드폰 전파 흉내에서 이 거리로 건다(binaural.rs).
    let mut field = field_mixer();
    field.recalculate_binaural_channel_azimuths();
    for rid in [1001, 1002, 1003] {
        let mut iso = isolated_mixer(rid);
        iso.recalculate_binaural_channel_azimuths();
        for (ch, r, _) in speakers() {
            if r == rid {
                assert_eq!(
                    field.binaural.debug_propagation_distance(ch),
                    iso.binaural.debug_propagation_distance(ch),
                    "ch{ch}(방 {rid}) 거리"
                );
                assert!(field.binaural.debug_propagation_distance(ch) > 0.0, "ch{ch} 거리가 0");
            }
        }
    }
}

#[test]
fn 궤적은_대상_방_스피커에만_나가고_pan_deg도_그_방_중심으로_돈다() {
    let traj = Trajectory {
        waypoints: vec![],
        current_position: p(3.0, 2.0, 1.5),
        target_room_zone_id: Some("1002".to_string()),
    };
    let run = |mut m: AudioMixer| {
        m.trajectory = Some(traj.clone());
        m.channel_pan_deg[7] = 90.0; // 중형 방 스피커 하나를 방 중심 기준으로 90° 돌린다
        let mut buf = vec![0.0f32; CH * 512];
        m.process(&mut buf, CH);
        m.channel_spatial_gains_target.clone()
    };

    let field = run(field_mixer());
    let iso = run(isolated_mixer(1002));

    for (ch, rid, _) in speakers() {
        if rid == 1002 {
            assert!(field[ch] > 0.0, "대상 방 ch{ch}에 소리가 없다");
            assert_eq!(field[ch], iso[ch], "대상 방 ch{ch} 게인");
        } else {
            assert_eq!(field[ch], 0.0, "다른 방(방 {rid}) ch{ch}로 소리가 샜다");
        }
    }
}
