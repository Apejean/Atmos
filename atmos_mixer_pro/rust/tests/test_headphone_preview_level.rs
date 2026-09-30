//! 헤드폰 미리듣기(바이노럴)의 전체 음량 기준.
//!
//! 사용자 보고(2026-09-28): "전체적으로 소리가 너무 작아졌다". 미리듣기가 거리(전파 감쇠)를
//! 흉내 내면서, 먼 스피커는 튜닝이 올려 둔 게인만큼 다시 작아졌다(테마 1의 CH2 13.8m:
//! 실제 트랙으로 -5.1dB). 거기에 HRTF 세트를 확산음장 평균 -6dB(표준 0dB)로 낮춰 둔 탓에
//! 기준 거리 정면 스피커조차 원음보다 5.9dB 작게 들렸다.
//!
//! 기준: 기준 거리(방 짧은 변의 절반)에 놓인 정면 스피커는 헤드폰에서 원음과 같은 크기로
//! 들린다(백색잡음, 두 귀 평균 파워). 방향·거리에 따른 차이는 그대로 남는다.

use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::common::config::{Point3D, RoomZone};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Arc;

const FS: u32 = 48_000;
const CH: usize = 12;
const BLOCK: usize = 512;

fn p(x: f32, y: f32, z: f32) -> Point3D {
    Point3D { x, y, z, ..Default::default() }
}

/// 15x20m 방(기준 거리 7.5m), 청취점 (7.5, 10, 1.6). `pos`의 CH1로 백색잡음을 틀고
/// (헤드폰 두 귀 평균 파워 / 원음 파워)를 dB로 돌려준다.
fn headphone_gain_db(pos: Point3D) -> f32 {
    let (gc_tx, _gc_rx) = crossbeam_channel::unbounded();
    let mut m = AudioMixer::new(FS, CH, BLOCK, gc_tx, None);
    for ch in 0..CH {
        GLOBAL_STATE.enabled_channels[ch].store(true, Ordering::SeqCst);
    }
    GLOBAL_STATE.hrtf_yaw.store(0.0f32.to_bits(), Ordering::Relaxed);
    m.startup_ramp.current_gain = 1.0;

    // 고정 시드 백색잡음(±0.1)
    let n = FS as usize * 4;
    let mut seed: u32 = 12_345;
    let samples: Vec<f32> = (0..n)
        .map(|_| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            ((seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5) * 0.2
        })
        .collect();
    let src_power = samples.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / n as f64;
    let data = Arc::new(SoundData { samples, channels: 1, sample_rate: FS });
    let mut inst = SoundInstance::new(
        1, 1, 1, "noise".into(), Some(data), None, FS, 1, true, 1.0, 0, false, None,
        GLOBAL_STATE.enabled_channels.len(),
    );
    inst.fade_weight = 1.0;
    m.instances[0] = Some(inst);

    m.room_zones = vec![RoomZone {
        room_id: 1,
        boundary_min: p(0.0, 0.0, 0.0),
        boundary_max: p(15.0, 20.0, 15.0),
        ear_level: 1.6,
        absorption_coeff: 0.3,
        ..Default::default()
    }];
    m.listener_position = Some(p(7.5, 10.0, 1.6));
    m.channel_positions = vec![None; CH];
    m.channel_room_ids = vec![None; CH];
    m.channel_positions[0] = Some(pos);
    m.channel_room_ids[0] = Some(1);
    m.binaural.enabled = true;
    m.set_binaural_room(Some(1));
    m.recalculate_binaural_channel_azimuths();

    let mut buf = vec![0.0f32; CH * BLOCK];
    for _ in 0..94 {
        buf.fill(0.0);
        m.process(&mut buf, CH);
    }
    let (mut power, mut k) = (0.0f64, 0usize);
    for _ in 0..188 {
        buf.fill(0.0);
        m.process(&mut buf, CH);
        for f in 0..BLOCK {
            power += ((buf[f * CH] as f64).powi(2) + (buf[f * CH + 1] as f64).powi(2)) * 0.5;
            k += 1;
        }
    }
    (10.0 * (power / k as f64 / src_power).log10()) as f32
}

#[test]
fn 기준_거리_정면_스피커는_헤드폰에서_원음과_같은_크기로_들린다() {
    let g = headphone_gain_db(p(7.5, 17.5, 1.6));
    assert!(g.abs() <= 1.0, "기준 거리 정면 스피커가 원음 대비 {g:+.1}dB로 들린다");
}

#[test]
fn 거리가_두_배면_헤드폰에서도_6db_작다() {
    // 전체 음량을 올려도 거리 차이(역제곱)는 그대로 남아야 한다.
    let near = headphone_gain_db(p(7.5, 15.0, 1.6));
    let far = headphone_gain_db(p(7.5, 20.0, 1.6));
    let d = near - far;
    assert!((d - 6.0).abs() <= 1.5, "5m와 10m 정면 스피커 차이가 {d:.1}dB다(기대 약 6dB)");
}
