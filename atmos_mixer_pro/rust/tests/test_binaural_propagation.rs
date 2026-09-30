//! 헤드폰 미리듣기(바이노럴)는 스피커→청취 지점 전파(거리만큼 늦게, 거리에 반비례해
//! 작게)를 흉내 내야 한다.
//!
//! 현장에서는 실제 공기가 이 지연·감쇠를 만들고, 자동 튜닝의 시간 정렬 딜레이와 거리 게인이
//! 그걸 상쇄해 모든 스피커가 청취 지점에 같은 시각·같은 크기로 도착한다. 예전 헤드폰
//! 미리듣기는 모든 스피커를 같은 거리에 둔 것처럼 렌더링해서, 보정만 남고 상쇄할 대상이
//! 없었다. 그래서 가까운 서브는 23ms 늦게·작게 들렸고, 크로스오버 부근이 서로 지워져
//! 저음이 약하게 들렸다(실기 보고: "저음 데시벨이 너무 낮아").

use rust_lib_atmos_mixer_pro::audio::acoustic::SPEED_OF_SOUND_M_S;
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::common::config::{Point3D, RoomZone};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Arc;

const FS: u32 = 48_000;
const CH: usize = 8;
const BLOCK: usize = 256;
const NEAR: usize = 2;
const FAR: usize = 3;
/// 15x20m 방: 자동 게인 기준 거리 = min(15, 20) / 2 = 7.5m.
const REF_M: f32 = 7.5;
/// 청취 지점(방 중앙, 귀 높이 1.2m) 정면으로 3.4m / 7.0m. 방위각이 같아 HRTF도 같다.
const NEAR_M: f32 = 3.4;
const FAR_M: f32 = 7.0;

fn p(x: f32, y: f32, z: f32) -> Point3D {
    Point3D { x, y, z, ..Default::default() }
}

fn mixer() -> AudioMixer {
    let (gc_tx, _gc_rx) = crossbeam_channel::unbounded();
    let mut m = AudioMixer::new(FS, CH, BLOCK, gc_tx, None);
    for ch in 0..CH {
        GLOBAL_STATE.enabled_channels[ch].store(true, Ordering::SeqCst);
    }
    GLOBAL_STATE.hrtf_yaw.store(0.0f32.to_bits(), Ordering::Relaxed);
    m.startup_ramp.current_gain = 1.0;
    m.room_zones = vec![RoomZone {
        room_id: 1,
        boundary_min: p(0.0, 0.0, 0.0),
        boundary_max: p(15.0, 20.0, 3.0),
        ear_level: 1.2,
        ..Default::default()
    }];
    m.listener_position = Some(p(7.5, 10.0, 1.2));
    m.channel_positions = vec![None; CH];
    m.channel_room_ids = vec![None; CH];
    m.channel_positions[NEAR] = Some(p(7.5, 10.0 + NEAR_M, 1.2));
    m.channel_positions[FAR] = Some(p(7.5, 10.0 + FAR_M, 1.2));
    m.channel_room_ids[NEAR] = Some(1);
    m.channel_room_ids[FAR] = Some(1);
    m.binaural.enabled = true;
    m.set_binaural_room(Some(1));
    m.recalculate_binaural_channel_azimuths();
    m
}

/// 바이노럴만 따로 돌린다: `ch`에 첫 샘플 임펄스를 넣고 헤드폰 L+R을 `blocks`블록 모은다.
fn binaural_impulse(m: &mut AudioMixer, ch: usize, blocks: usize) -> Vec<f32> {
    let mut out = Vec::new();
    for b in 0..blocks {
        let mut buf = vec![0.0f32; CH * BLOCK];
        if b == 0 {
            buf[ch] = 1.0;
        }
        m.binaural.process_interleaved(&mut buf, CH);
        for f in 0..BLOCK {
            out.push(buf[f * CH] + buf[f * CH + 1]);
        }
    }
    out
}

fn peak(x: &[f32]) -> (usize, f32) {
    x.iter()
        .enumerate()
        .fold((0, 0.0f32), |(bi, bv), (i, v)| if v.abs() > bv { (i, v.abs()) } else { (bi, bv) })
}

fn db(x: f32) -> f32 {
    20.0 * x.max(1e-12).log10()
}

#[test]
fn 헤드폰에서_먼_스피커는_거리만큼_늦게_거리에_반비례해_작게_들린다() {
    let (near_i, near_v) = peak(&binaural_impulse(&mut mixer(), NEAR, 12));
    let (far_i, far_v) = peak(&binaural_impulse(&mut mixer(), FAR, 12));

    let expected_gap = (FAR_M - NEAR_M) / SPEED_OF_SOUND_M_S * FS as f32;
    let gap = far_i as f32 - near_i as f32;
    assert!(
        (gap - expected_gap).abs() <= 2.0,
        "도착 시각 차이 {gap}샘플, 거리 차이로는 {expected_gap:.0}샘플이어야 한다"
    );
    let level_db = db(far_v / near_v);
    let expected_db = db(NEAR_M / FAR_M);
    assert!(
        (level_db - expected_db).abs() < 0.5,
        "먼 스피커가 {level_db:+.2}dB, 거리 비로는 {expected_db:+.2}dB여야 한다"
    );
    // 기준 거리(7.5m)에서 0dB: 가까운 스피커는 그만큼 커진다.
    let near_ref_db = db(near_v / peak(&binaural_impulse(&mut mixer_at(REF_M), NEAR, 12)).1);
    assert!(
        (near_ref_db - db(REF_M / NEAR_M)).abs() < 0.5,
        "기준 거리 대비 가까운 스피커 {near_ref_db:+.2}dB, {:+.2}dB여야 한다",
        db(REF_M / NEAR_M)
    );
}

/// NEAR 채널을 청취 지점 정면 `dist_m`에 둔 믹서.
fn mixer_at(dist_m: f32) -> AudioMixer {
    let mut m = mixer();
    m.channel_positions[NEAR] = Some(p(7.5, 10.0 + dist_m, 1.2));
    m.recalculate_binaural_channel_azimuths();
    m
}

/// 믹서 전체(채널 DSP 포함)로 `ch`에 임펄스를 흘려 헤드폰 L+R을 모은다.
fn mixer_impulse(m: &mut AudioMixer, ch: usize, blocks: usize) -> Vec<f32> {
    // 딜레이·게인 스무딩이 자리 잡을 때까지 무음으로 돌린다.
    let mut buf = vec![0.0f32; CH * BLOCK];
    for _ in 0..40 {
        buf.fill(0.0);
        m.process(&mut buf, CH);
    }
    let mut samples = vec![0.0f32; FS as usize];
    samples[0] = 0.5;
    let data = Arc::new(SoundData { samples, channels: 1, sample_rate: FS });
    let mut inst = SoundInstance::new(
        1, 1, 1, "impulse".to_string(), Some(data), None, FS, 1, false, 1.0, ch, false, None,
        GLOBAL_STATE.enabled_channels.len(),
    );
    inst.fade_weight = 1.0;
    m.instances[0] = Some(inst);
    let mut out = Vec::new();
    for _ in 0..blocks {
        buf.fill(0.0);
        m.process(&mut buf, CH);
        for f in 0..BLOCK {
            out.push(buf[f * CH] + buf[f * CH + 1]);
        }
    }
    out
}

#[test]
fn 자동_정렬_딜레이와_거리_게인을_걸면_헤드폰에서도_같은_시각_같은_크기로_도착한다() {
    // Dart 자동 튜닝과 같은 값: 딜레이 = (가장 먼 거리 - 내 거리)/c, 게인 = 20log10(거리/기준).
    let tuned = |ch: usize| {
        let mut m = mixer();
        let delay_ms = (FAR_M - NEAR_M) / SPEED_OF_SOUND_M_S * 1000.0;
        m.channel_dsp[NEAR].update_delay_target(delay_ms);
        m.channel_dsp[NEAR].set_gain_db(db(NEAR_M / REF_M));
        m.channel_dsp[FAR].update_delay_target(0.0);
        m.channel_dsp[FAR].set_gain_db(db(FAR_M / REF_M));
        peak(&mixer_impulse(&mut m, ch, 16))
    };
    let (near_i, near_v) = tuned(NEAR);
    let (far_i, far_v) = tuned(FAR);
    assert!(
        (near_i as i64 - far_i as i64).abs() <= 2,
        "보정한 두 스피커의 도착 시각이 {}샘플 어긋난다",
        near_i as i64 - far_i as i64
    );
    let diff = db(near_v / far_v);
    assert!(diff.abs() < 0.5, "보정한 두 스피커의 크기가 {diff:+.2}dB 다르다");
}

#[test]
fn 스피커를_옮겨도_헤드폰_출력에_딸깍_소리가_나지_않는다() {
    let mut m = mixer();
    let tone = |n: usize| 0.3 * (std::f32::consts::TAU * 200.0 * n as f32 / FS as f32).sin();
    let mut n = 0usize;
    let mut run = |m: &mut AudioMixer, blocks: usize| {
        let mut out = Vec::new();
        for _ in 0..blocks {
            let mut buf = vec![0.0f32; CH * BLOCK];
            for f in 0..BLOCK {
                buf[f * CH + NEAR] = tone(n);
                n += 1;
            }
            m.binaural.process_interleaved(&mut buf, CH);
            for f in 0..BLOCK {
                out.push(buf[f * CH]);
            }
        }
        out
    };
    run(&mut m, 40);
    let before = run(&mut m, 20);
    // 2m 뒤로 옮긴다(지연 +5.9ms, 크기 -4.5dB).
    m.channel_positions[NEAR] = Some(p(7.5, 10.0 + NEAR_M + 2.0, 1.2));
    m.recalculate_binaural_channel_azimuths();
    let transition = run(&mut m, 20);
    let after = run(&mut m, 20);
    let max_step = |x: &[f32]| x.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max);
    let reference = max_step(&before).max(max_step(&after));
    let tr = max_step(&transition);
    assert!(
        tr <= reference * 1.5,
        "옮기는 순간 불연속: 전환 {tr:.5}, 정상 {reference:.5}"
    );
}

/// NEAR에 3kHz 사인을 넣으며 헤드폰 L을 모은다. 블록마다 `step(블록 번호)`를 먼저 부른다.
fn tone_run(m: &mut AudioMixer, n0: &mut usize, blocks: usize, mut step: impl FnMut(&mut AudioMixer, usize)) -> Vec<f32> {
    let mut out = Vec::new();
    for b in 0..blocks {
        step(m, b);
        let mut buf = vec![0.0f32; CH * BLOCK];
        for f in 0..BLOCK {
            buf[f * CH + NEAR] =
                0.3 * (std::f32::consts::TAU * 3000.0 * (*n0 + f) as f32 / FS as f32).sin();
        }
        *n0 += BLOCK;
        m.binaural.process_interleaved(&mut buf, CH);
        for f in 0..BLOCK {
            out.push(buf[f * CH]);
        }
    }
    out
}

/// 5ms 구간 RMS(dB)들.
fn window_db(x: &[f32]) -> Vec<f32> {
    x.chunks(240)
        .map(|w| db((w.iter().map(|v| v * v).sum::<f32>() / w.len() as f32).sqrt()))
        .collect()
}

#[test]
fn 스피커를_끄는_동안에도_헤드폰_소리가_끊기지_않는다() {
    // 드래그: 위치가 16ms(3블록)마다 5cm씩 멀어진다(약 3m/s). 예전에는 전파 지연을 그때마다
    // 교차 페이드해서, 두 지연이 섞이는 순간 3kHz가 서로 지워져 소리가 뚝뚝 끊겼다(실기 보고).
    // 시간 정렬 딜레이처럼 움직이는 동안에는 지연을 붙잡아 두고, 크기만 부드럽게 따라간다.
    let mut m = mixer();
    let mut n = 0usize;
    tone_run(&mut m, &mut n, 40, |_, _| {});
    let mut dist = NEAR_M;
    let drag = tone_run(&mut m, &mut n, 180, |m, b| {
        if b % 3 == 0 {
            dist += 0.05;
            m.channel_positions[NEAR] = Some(p(7.5, 10.0 + dist, 1.2));
            m.recalculate_binaural_channel_azimuths();
        }
    });
    let w = window_db(&drag);
    let worst = w.windows(2).map(|p| (p[1] - p[0]).abs()).fold(0.0f32, f32::max);
    assert!(worst < 0.5, "끄는 동안 5ms 사이 크기가 {worst:.1}dB 튀었다(끊김)");
}

#[test]
fn 끌기를_멈추면_새_거리의_지연으로_옮겨_간다() {
    let mut m = mixer();
    let mut n = 0usize;
    tone_run(&mut m, &mut n, 10, |_, _| {});
    // 1m씩 세 번 끌고(16ms 간격) 놓는다.
    let mut dist = NEAR_M;
    tone_run(&mut m, &mut n, 9, |m, b| {
        if b % 3 == 0 {
            dist += 1.0;
            m.channel_positions[NEAR] = Some(p(7.5, 10.0 + dist, 1.2));
            m.recalculate_binaural_channel_azimuths();
        }
    });
    // 놓고 0.5초 뒤에는 새 거리의 지연이어야 한다: 임펄스 도착 시각으로 확인.
    tone_run(&mut m, &mut n, 94, |_, _| {});
    // 지연 버퍼·HRIR 꼬리에 남은 사인을 무음으로 비운 뒤 잰다.
    for _ in 0..30 {
        let mut buf = vec![0.0f32; CH * BLOCK];
        m.binaural.process_interleaved(&mut buf, CH);
    }
    let mut fresh = mixer();
    fresh.channel_positions[NEAR] = Some(p(7.5, 10.0 + dist, 1.2));
    fresh.recalculate_binaural_channel_azimuths();
    let (expected_i, _) = peak(&binaural_impulse(&mut fresh, NEAR, 16));
    let (got_i, _) = peak(&binaural_impulse(&mut m, NEAR, 16));
    assert!(
        (got_i as i64 - expected_i as i64).abs() <= 2,
        "놓은 뒤 지연이 새 거리({dist:.1}m)로 옮겨 가지 않았다: 도착 {got_i}, 기대 {expected_i}"
    );
}

#[test]
fn 실제_스피커로_나가는_채널은_헤드폰_거리_흉내의_영향을_받지_않는다() {
    // 바이노럴은 CH1/CH2(헤드폰 L/R)만 덮어쓰고, 나머지 채널은 스피커 출력 그대로다.
    let mut m = mixer();
    let mut buf = vec![0.0f32; CH * BLOCK];
    for f in 0..BLOCK {
        buf[f * CH + NEAR] = 0.1 * (f as f32 * 0.05).sin();
        buf[f * CH + FAR] = 0.2 * (f as f32 * 0.03).cos();
    }
    let input = buf.clone();
    m.binaural.process_interleaved(&mut buf, CH);
    for f in 0..BLOCK {
        for ch in 2..CH {
            assert_eq!(buf[f * CH + ch], input[f * CH + ch], "CH{} 스피커 출력이 바뀌었다", ch + 1);
        }
    }
}

#[test]
fn 바이노럴을_다시_켜도_예전_소리가_지연_버퍼에서_나오지_않는다() {
    let mut m = mixer();
    for _ in 0..4 {
        let mut buf = vec![0.0f32; CH * BLOCK];
        for f in 0..BLOCK {
            buf[f * CH + FAR] = 0.5;
        }
        m.binaural.process_interleaved(&mut buf, CH);
    }
    m.binaural.enabled = false;
    let mut buf = vec![0.0f32; CH * BLOCK];
    m.binaural.process_interleaved(&mut buf, CH);
    m.binaural.enabled = true;
    let mut worst = 0.0f32;
    for b in 0..12 {
        let mut buf = vec![0.0f32; CH * BLOCK];
        m.binaural.process_interleaved(&mut buf, CH);
        // 첫 3블록은 HRTF 컨볼루션 꼬리(IR 512샘플, 기존 동작)라 건너뛴다. 지연 버퍼에 남은
        // 옛 소리라면 FAR 거리(7m ≈ 990샘플)만큼 뒤에 나오므로 이후 블록에서 잡힌다.
        if b < 3 {
            continue;
        }
        for f in 0..BLOCK {
            worst = worst.max(buf[f * CH].abs()).max(buf[f * CH + 1].abs());
        }
    }
    assert!(worst < 1e-6, "다시 켜자 지난 소리가 나왔다: {worst:.2e}");
}
