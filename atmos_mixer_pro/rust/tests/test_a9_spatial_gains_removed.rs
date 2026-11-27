//! A9 정리(2026-11-27 사용자 결정 "정리해도돼"): 채널별 공간 게인(방 볼륨·전역 궤적·pan_deg로 계산한 DBAP 가중치)은
//! 계산만 하고 출력에 쓰지 않았고, 위치 명령이 부르던 `recalculate_spatial_dsp`도 계산만 하고 결과를 버렸다. 그런데
//! 그 계산이 오디오 콜백 안에서 설정 잠금(`RwLock::try_read`)을 잡고 벡터를 복제했다(3법칙 1·2). 그래서 지웠다.
//! 이 시험은 (1) 지운 것이 다시 들어오지 않는지 소스로 보고, (2) 궤적 위치가 바뀌어도 출력이 같다는 것을 본다.
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::common::config::{Point3D, RoomZone, Trajectory};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Arc;

const CH: usize = 4;
const BLOCK: usize = 512;

fn p(x: f32, y: f32, z: f32) -> Point3D {
    Point3D { x, y, z, ..Default::default() }
}

#[test]
fn 지운_공간_게인_계산과_pan_audience_pos가_소스에_없다() {
    let root = env!("CARGO_MANIFEST_DIR");
    let read = |rel: &str| std::fs::read_to_string(format!("{root}/{rel}")).unwrap();
    for (file, words) in [
        ("src/audio/mixer.rs", &["channel_spatial_gains", "channel_pan_deg", "recalculate_spatial_dsp", "temp_spatial_weights"][..]),
        ("src/audio/engine.rs", &["recalculate_spatial_dsp", "SetChannelPanDeg"][..]),
        ("src/core/state.rs", &["spatial_gains"][..]),
        ("src/api/simple.rs", &["fn api_get_spatial_gains", "fn api_set_channel_pan_deg"][..]),
        ("src/common/commands.rs", &["SetChannelPanDeg"][..]),
        ("src/osc/listener.rs", &["/audience/pos"][..]),
    ] {
        let src = read(file);
        for w in words {
            assert!(!src.contains(w), "{file}에 지운 `{w}`가 남아 있다");
        }
    }
}

fn mixer() -> AudioMixer {
    let (gc_tx, _gc_rx) = crossbeam_channel::unbounded();
    let mut m = AudioMixer::new(48_000, CH, BLOCK, gc_tx, None);
    m.startup_ramp.current_gain = 1.0;
    m.room_zones = vec![RoomZone {
        room_id: 1,
        boundary_min: p(0.0, 0.0, 0.0),
        boundary_max: p(6.0, 4.0, 3.0),
        ..Default::default()
    }];
    m.channel_positions = vec![Some(p(1.0, 1.0, 2.0)), Some(p(5.0, 1.0, 2.0)), Some(p(1.0, 3.0, 2.0)), Some(p(5.0, 3.0, 2.0))];
    m.channel_room_ids = vec![Some(1); CH];
    let data = Arc::new(SoundData {
        samples: (0..48_000 * 2).map(|i| 0.2 * ((i as f32) * 0.03).sin()).collect(),
        channels: 1,
        sample_rate: 48_000,
    });
    m.instances[0] = Some(SoundInstance::new(1, 1, 1, "t".into(), Some(data), None, 48_000, 1, true, 1.0, 0, false, None, CH));
    m
}

fn render(m: &mut AudioMixer, moves: &[Option<Point3D>]) -> Vec<f32> {
    let mut all = Vec::new();
    for pos in moves {
        if let Some(pos) = pos {
            m.trajectory = Some(Trajectory { waypoints: vec![], current_position: pos.clone(), target_room_zone_id: None });
        }
        let mut buf = vec![0.0f32; CH * BLOCK];
        m.process(&mut buf, CH);
        all.extend_from_slice(&buf);
    }
    all
}

#[test]
fn 궤적_위치가_바뀌어도_출력은_같다() {
    for ch in 0..CH {
        GLOBAL_STATE.enabled_channels[ch].store(true, Ordering::SeqCst);
        GLOBAL_STATE.mono_rows_open[ch].store(true, Ordering::SeqCst);
    }
    let still = render(&mut mixer(), &vec![None; 8]);
    let moving: Vec<Option<Point3D>> = (0..8).map(|i| Some(p(0.5 + i as f32 * 0.7, 2.0, 1.5))).collect();
    let moved = render(&mut mixer(), &moving);
    assert!(still.iter().any(|v| v.abs() > 0.01), "소리가 나지 않아 비교가 의미 없다");
    assert_eq!(still, moved, "궤적 위치(/audience/pos·추적 OSC가 바꾸던 값)가 출력을 바꿨다");
}
