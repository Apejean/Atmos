//! 헤드폰 미리듣기(바이노럴)는 **지금 보고 있는 방**의 스피커만 들려줘야 한다.
//!
//! 설계는 방 단위로 한다. 예전에는 모든 방의 스피커가 한꺼번에 헤드폰으로 나와서,
//! 다섯 방에 동시에 서 있는 소리가 됐다. 방이 정해지지 않았으면(방을 안 만든 경우)
//! 예전처럼 전체 채널을 렌더링한다.

use rust_lib_atmos_mixer_pro::audio::bass_route::compute_bass_route;
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::common::config::{Point3D, RoomZone};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;

const CH: usize = 8;
const BLOCK: usize = 256;
const ROOM_A: u32 = 11;
const ROOM_B: u32 = 22;
/// 방 A의 스피커 채널(여기에만 신호를 넣는다).
const CH_IN_A: usize = 2;
/// 방 B의 서브우퍼 채널(서브 테스트용).
const SUB_IN_B: usize = 4;

fn point(x: f32, y: f32, z: f32) -> Point3D {
    Point3D { x, y, z, ..Default::default() }
}

fn zone(room_id: u32, w: f32, d: f32) -> RoomZone {
    RoomZone {
        room_id,
        boundary_min: point(0.0, 0.0, 0.0),
        boundary_max: point(w, d, 3.0),
        ear_level: 1.2,
        ..Default::default()
    }
}

/// `signal_ch`에만 `freq` 사인을 계속 넣으며 바이노럴을 돌려 헤드폰 L/R 최대값을 잰다.
/// 헤드폰 미리듣기는 스피커→청취 지점 거리만큼 늦게 들리므로(방 A 스피커는 약 8m ≈ 1200샘플)
/// 한 블록만 보면 아직 도착 전이다. 12블록(3072샘플)을 본다.
fn headphone_peak(mixer: &mut AudioMixer, signal_ch: usize, freq: f32) -> f32 {
    let mut peak = 0.0f32;
    for block in 0..12 {
        let mut buf = vec![0.0f32; CH * BLOCK];
        for frame in 0..BLOCK {
            let t = (block * BLOCK + frame) as f32 / 48000.0;
            buf[frame * CH + signal_ch] = 0.5 * (std::f32::consts::TAU * freq * t).sin();
        }
        mixer.binaural.process_interleaved(&mut buf, CH);
        for frame in 0..BLOCK {
            peak = peak.max(buf[frame * CH].abs()).max(buf[frame * CH + 1].abs());
        }
    }
    peak
}

/// 한 채널에만 신호를 넣고 바이노럴을 돌려 L/R 출력 크기를 잰다.
/// `signal_channel_room`이 None이면 그 채널은 방이 지정되지 않은 예전 스피커다.
fn binaural_output_level_for(
    active_room: Option<u32>,
    signal_channel_room: Option<u32>,
) -> f32 {
    let (gc_tx, _) = crossbeam_channel::unbounded();
    let mut mixer = AudioMixer::new(48000, CH, BLOCK, gc_tx, None);
    for ch in 0..CH {
        GLOBAL_STATE.enabled_channels[ch].store(true, Ordering::SeqCst);
    }
    GLOBAL_STATE.hrtf_yaw.store(0.0f32.to_bits(), Ordering::Relaxed);

    mixer.room_zones = vec![zone(ROOM_A, 20.0, 15.0), zone(ROOM_B, 6.0, 4.0)];
    mixer.channel_positions = vec![None; CH];
    mixer.channel_room_ids = vec![None; CH];
    mixer.channel_positions[CH_IN_A] = Some(point(3.0, 3.0, 1.5));
    mixer.channel_room_ids[CH_IN_A] = signal_channel_room;
    mixer.binaural.enabled = true;
    mixer.set_binaural_room(active_room);
    mixer.recalculate_binaural_channel_azimuths();

    headphone_peak(&mut mixer, CH_IN_A, 440.0)
}

#[test]
fn 방이_지정되지_않은_예전_스피커는_어느_방에서나_들린다() {
    // 3D 화면도 방이 없는 스피커를 모든 방에서 보여준다(dynamic_3d_room.dart).
    // 헤드폰만 음소거하면 기준이 어긋나고, 예전 프로젝트의 스피커가 갑자기 사라진다.
    let level = binaural_output_level_for(Some(ROOM_B), None);
    assert!(
        level > 0.01,
        "방이 없는 스피커가 헤드폰에서 음소거됐다: {level:.4}"
    );
}

/// 방 A의 한 채널에만 신호를 넣고 바이노럴을 돌려 L/R 출력 크기를 잰다.
fn binaural_output_level(active_room: Option<u32>) -> f32 {
    let (gc_tx, _) = crossbeam_channel::unbounded();
    let mut mixer = AudioMixer::new(48000, CH, BLOCK, gc_tx, None);
    for ch in 0..CH {
        GLOBAL_STATE.enabled_channels[ch].store(true, Ordering::SeqCst);
    }
    GLOBAL_STATE.hrtf_yaw.store(0.0f32.to_bits(), Ordering::Relaxed);

    mixer.room_zones = vec![zone(ROOM_A, 20.0, 15.0), zone(ROOM_B, 6.0, 4.0)];
    mixer.channel_positions = vec![None; CH];
    mixer.channel_room_ids = vec![None; CH];
    mixer.channel_positions[CH_IN_A] = Some(point(3.0, 3.0, 1.5));
    mixer.channel_room_ids[CH_IN_A] = Some(ROOM_A);
    mixer.channel_positions[4] = Some(point(1.0, 1.0, 1.5));
    mixer.channel_room_ids[4] = Some(ROOM_B);
    mixer.binaural.enabled = true;
    mixer.set_binaural_room(active_room);
    mixer.recalculate_binaural_channel_azimuths();

    // 믹서의 채널 DSP/라우팅을 거치지 않고 바이노럴 입력만 보기 위해 직접 넣는다.
    headphone_peak(&mut mixer, CH_IN_A, 440.0)
}

#[test]
fn 보고_있는_방의_스피커만_헤드폰으로_나온다() {
    let in_room_a = binaural_output_level(Some(ROOM_A));
    let in_room_b = binaural_output_level(Some(ROOM_B));

    assert!(in_room_a > 0.01, "방 A를 보고 있는데 방 A 스피커가 안 들린다: {in_room_a:.4}");
    assert!(
        in_room_b < 1e-6,
        "방 B를 보고 있는데 방 A 스피커가 들린다: {in_room_b:.6}"
    );
}

/// 방 A 서브(CH_IN_A)와 방 B 서브(SUB_IN_B)를 지정하고, `signal_ch` 서브에만 60Hz를 넣어
/// `active_room`을 보고 있을 때 헤드폰 L/R 크기를 잰다.
fn sub_level_in_headphones(active_room: u32, signal_ch: usize) -> f32 {
    let (gc_tx, _) = crossbeam_channel::unbounded();
    let mut mixer = AudioMixer::new(48000, CH, BLOCK, gc_tx, None);
    for ch in 0..CH {
        GLOBAL_STATE.enabled_channels[ch].store(true, Ordering::SeqCst);
    }
    GLOBAL_STATE.hrtf_yaw.store(0.0f32.to_bits(), Ordering::Relaxed);
    mixer.room_zones = vec![zone(ROOM_A, 20.0, 15.0), zone(ROOM_B, 6.0, 4.0)];
    mixer.channel_positions = vec![None; CH];
    mixer.channel_room_ids = vec![None; CH];
    mixer.channel_positions[CH_IN_A] = Some(point(3.0, 3.0, 1.5));
    mixer.channel_room_ids[CH_IN_A] = Some(ROOM_A);
    mixer.channel_positions[SUB_IN_B] = Some(point(1.0, 1.0, 1.5));
    mixer.channel_room_ids[SUB_IN_B] = Some(ROOM_B);
    // 엔진은 공간 설정 payload로 받는다(simple.rs → UpdateSpatialConfig).
    let is_sub: Vec<bool> = (0..CH).map(|c| c == CH_IN_A || c == SUB_IN_B).collect();
    let has_speaker: Vec<bool> = mixer.channel_positions.iter().map(|p| p.is_some()).collect();
    let route = compute_bass_route(&is_sub, &mixer.channel_room_ids, &has_speaker);
    let _old = mixer.set_bass_routing(route, is_sub);
    mixer.binaural.enabled = true;
    // 새 라우팅 표는 블록 경계에서 바꿔 끼운다.
    let mut warm = vec![0.0f32; CH * BLOCK];
    mixer.process(&mut warm, CH);
    mixer.set_binaural_room(Some(active_room));
    mixer.recalculate_binaural_channel_azimuths();

    headphone_peak(&mut mixer, signal_ch, 60.0)
}

#[test]
fn 서브는_자기_방을_볼_때만_헤드폰에_들린다() {
    // 방별 베이스 매니지먼트: 방마다 저역이 그 방 서브로 모인다. 보고 있는 방의 서브를
    // 빼면 미리듣기에서 저음이 통째로 사라지고(소리가 날카롭게 들린다), 다른 방 서브를
    // 넣으면 그 방 저역이 섞인다.
    let own = sub_level_in_headphones(ROOM_A, CH_IN_A);
    let other = sub_level_in_headphones(ROOM_A, SUB_IN_B);
    let other_own = sub_level_in_headphones(ROOM_B, SUB_IN_B);

    assert!(own > 0.01, "방 A를 보는데 방 A 서브가 헤드폰에서 빠졌다: {own:.4}");
    assert!(other < 1e-6, "방 A를 보는데 방 B 서브가 헤드폰에 섞였다: {other:.6}");
    assert!(other_own > 0.01, "방 B를 보는데 방 B 서브가 헤드폰에서 빠졌다: {other_own:.4}");
}

/// CH_IN_A에만 스피커를 두고(`placed`가 false면 아무 스피커도 없음) 스피커 없는 채널 5에
/// 신호를 넣어 헤드폰 크기를 잰다.
fn unplaced_channel_level(placed: bool) -> f32 {
    const UNPLACED: usize = 5;
    let (gc_tx, _) = crossbeam_channel::unbounded();
    let mut mixer = AudioMixer::new(48000, CH, BLOCK, gc_tx, None);
    for ch in 0..CH {
        GLOBAL_STATE.enabled_channels[ch].store(true, Ordering::SeqCst);
    }
    GLOBAL_STATE.hrtf_yaw.store(0.0f32.to_bits(), Ordering::Relaxed);
    mixer.room_zones = vec![zone(ROOM_A, 20.0, 15.0)];
    mixer.channel_positions = vec![None; CH];
    mixer.channel_room_ids = vec![None; CH];
    if placed {
        mixer.channel_positions[CH_IN_A] = Some(point(3.0, 3.0, 1.5));
        mixer.channel_room_ids[CH_IN_A] = Some(ROOM_A);
    }
    mixer.binaural.enabled = true;
    mixer.set_binaural_room(Some(ROOM_A));
    mixer.recalculate_binaural_channel_azimuths();
    headphone_peak(&mut mixer, UNPLACED, 440.0)
}

#[test]
fn 스피커를_배치하지_않은_채널은_헤드폰에서_들리지_않는다() {
    // 설계에 없는 채널(예: 6채널 장비의 CH3~6)이 어느 방에서나 정면에서 들리면 헷갈린다.
    let level = unplaced_channel_level(true);
    assert!(level < 1e-6, "스피커 없는 채널이 헤드폰에 섞였다: {level:.6}");
}

#[test]
fn 스피커가_하나도_없으면_모든_채널을_정면으로_들려준다() {
    // 아직 스피커를 하나도 배치하지 않은 프로젝트에서 헤드폰이 무음이 되지 않게 한다.
    let level = unplaced_channel_level(false);
    assert!(level > 0.01, "스피커가 없는 프로젝트에서 헤드폰이 무음이다: {level:.4}");
}

#[test]
fn 방이_정해지지_않으면_전체_채널을_그대로_렌더링한다() {
    let all = binaural_output_level(None);
    assert!(all > 0.01, "방 지정이 없을 때 소리가 사라졌다: {all:.4}");
}
