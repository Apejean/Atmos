//! 헤드폰 미리듣기(바이노럴)는 현장 물리를 흉내 내고, 현장(스피커 출력)에는 흉내를 넣지 않는다.
//!
//! 자동 튜닝은 현장 물리(공기 흡음, 경계면 저음 증가, 룸 모드 등)를 **보정**한다. 현장에서는
//! 실제 물리가 그 보정을 상쇄하고, 헤드폰에서는 흉내 낸 물리가 상쇄해야 미리듣기가 현장과 같다.
//! - 공기 흡음: 예전에는 채널 DSP에 걸려 현장 스피커 출력에도 들어갔다(실제 공기와 이중 감쇠).
//! - 높이: 예전에는 모든 스피커를 귀 높이(고도 0°) HRTF로 렌더링했다.
//! - 현장 물리 밴드(경계면·룸 모드·근접면 반사·지향성): Dart position_eq.dart가 자동 EQ와 같은
//!   모델로 계산해 보내고, 엔진은 헤드폰 경로에만 건다.

use rust_lib_atmos_mixer_pro::audio::binaural::MAX_SIM_BANDS;
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, EqBand, EqType, Point3D, RoomZone};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Arc;

const FS: u32 = 48_000;
const CH: usize = 8;
const BLOCK: usize = 256;
const SPK: usize = 2;
const LISTENER: (f32, f32, f32) = (20.0, 20.0, 1.2);

fn p(x: f32, y: f32, z: f32) -> Point3D {
    Point3D { x, y, z, ..Default::default() }
}

fn db(x: f32) -> f32 {
    20.0 * x.max(1e-12).log10()
}

/// 40x40m(천장 30m) 방, 청취 지점 (20, 20, 1.2)에 스피커 하나(SPK).
fn mixer_with_speaker(pos: Point3D, binaural: bool) -> AudioMixer {
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
        boundary_max: p(40.0, 40.0, 30.0),
        ear_level: LISTENER.2,
        ..Default::default()
    }];
    m.listener_position = Some(p(LISTENER.0, LISTENER.1, LISTENER.2));
    m.channel_positions = vec![None; CH];
    m.channel_room_ids = vec![None; CH];
    m.channel_positions[SPK] = Some(pos);
    m.channel_room_ids[SPK] = Some(1);
    m.binaural.enabled = binaural;
    m.set_binaural_room(Some(1));
    m.recalculate_binaural_channel_azimuths();
    m
}

/// 바이노럴만 돌린다: SPK에 `freq` 사인을 계속 넣고, 마지막 8블록의 (L, R) RMS.
fn headphone_lr(m: &mut AudioMixer, freq: f32, blocks: usize) -> (f32, f32) {
    let (mut l, mut r, mut n) = (0.0f64, 0.0f64, 0usize);
    for b in 0..blocks {
        let mut buf = vec![0.0f32; CH * BLOCK];
        for f in 0..BLOCK {
            let t = (b * BLOCK + f) as f32 / FS as f32;
            buf[f * CH + SPK] = 0.3 * (std::f32::consts::TAU * freq * t).sin();
        }
        m.binaural.process_interleaved(&mut buf, CH);
        if b + 8 >= blocks {
            for f in 0..BLOCK {
                l += (buf[f * CH] as f64).powi(2);
                r += (buf[f * CH + 1] as f64).powi(2);
                n += 1;
            }
        }
    }
    ((l / n as f64).sqrt() as f32, (r / n as f64).sqrt() as f32)
}

fn headphone_level(m: &mut AudioMixer, freq: f32) -> f32 {
    let (l, r) = headphone_lr(m, freq, 30);
    (l * l + r * r).sqrt()
}

#[test]
fn 공기_흡음은_현장_스피커_출력에는_걸리지_않는다() {
    // 공간 DSP 재계산(recalculate_spatial_dsp)은 엔진 설정이 있어야 돈다(앱과 같은 조건).
    if let Ok(mut c) = GLOBAL_STATE.config.write() {
        if c.is_none() {
            *c = Some(AppConfig::default());
        }
    }
    let level = |freq: f32| {
        let mut m = mixer_with_speaker(p(LISTENER.0, LISTENER.1 + 20.0, LISTENER.2), false);
        m.recalculate_spatial_dsp();
        let n = FS as usize * 4;
        let samples: Vec<f32> = (0..n)
            .map(|i| 0.1 * (std::f32::consts::TAU * freq * i as f32 / FS as f32).sin())
            .collect();
        let data = Arc::new(SoundData { samples, channels: 1, sample_rate: FS });
        let mut inst = SoundInstance::new(
            1, 1, 1, "tone".to_string(), Some(data), None, FS, 1, true, 1.0, SPK, false, None,
            GLOBAL_STATE.enabled_channels.len(),
        );
        inst.fade_weight = 1.0;
        m.instances[0] = Some(inst);
        let mut buf = vec![0.0f32; CH * BLOCK];
        let mut acc = 0.0f64;
        let mut cnt = 0usize;
        for b in 0..200 {
            buf.fill(0.0);
            m.process(&mut buf, CH);
            if b >= 150 {
                for f in 0..BLOCK {
                    acc += (buf[f * CH + SPK] as f64).powi(2);
                    cnt += 1;
                }
            }
        }
        (acc / cnt as f64).sqrt() as f32
    };
    // 20m 떨어진 스피커: 실제 공기가 고역을 깎으므로 출력에서 또 깎으면 안 된다.
    let diff = db(level(16000.0) / level(1000.0));
    assert!(
        diff.abs() < 0.5,
        "20m 스피커 출력의 16kHz가 1kHz 대비 {diff:+.1}dB — 현장 출력에 공기 흡음이 걸렸다"
    );
}

#[test]
fn 헤드폰에서는_먼_스피커의_고역이_공기_흡음만큼_줄어든다() {
    // 같은 방향(정면), 거리만 다르다. 거리 감쇠(1/r)는 주파수와 무관하므로 비율로 본다.
    let hf_ratio = |dist: f32| {
        let pos = p(LISTENER.0, LISTENER.1 + dist, LISTENER.2);
        let hf = headphone_level(&mut mixer_with_speaker(pos.clone(), true), 16000.0);
        let mf = headphone_level(&mut mixer_with_speaker(pos, true), 1000.0);
        hf / mf
    };
    let near = hf_ratio(2.0);
    let far = hf_ratio(18.0);
    let diff = db(far / near);
    // 공기 흡음 모델(AirAbsorptionFilter) 자체의 18m/2m 응답과 같아야 한다.
    let model = db(air_model_level(18.0, 16000.0) / air_model_level(2.0, 16000.0));
    assert!(model < -1.5, "공기 흡음 모델이 16kHz를 {model:+.1}dB밖에 안 깎는다");
    assert!(
        (diff - model).abs() < 0.5,
        "18m 스피커의 고역이 2m 대비 {diff:+.1}dB, 공기 흡음 모델은 {model:+.1}dB — 헤드폰에 모델대로 걸리지 않았다"
    );
}

#[test]
fn 공기_흡음_감쇠량은_거리가_뛰어도_계단처럼_바뀌지_않는다() {
    // 헤드폰 경로는 스피커를 끄는 동안 payload마다 새 거리를 바로 넘긴다. 차단 주파수만
    // 스무딩하고 셸프 감쇠량을 목표 거리에서 바로 구하면 감쇠량이 그 자리에서 뛴다(Law 3).
    // 10m로 안정된 두 필터 중 하나만 30m로 바꾸고, 두 출력 차이가 첫 5샘플에서 이미
    // 최종 차이의 큰 몫이면 계단이다.
    use rust_lib_atmos_mixer_pro::audio::dsp::acoustic_physics::AirAbsorptionFilter;
    let (mut a, mut b) = (AirAbsorptionFilter::new(), AirAbsorptionFilter::new());
    let x = |i: usize| (std::f32::consts::TAU * 12_000.0 * i as f32 / FS as f32).sin();
    let settle = FS as usize / 2;
    for i in 0..settle {
        a.set_distance(10.0, FS as f32);
        b.set_distance(10.0, FS as f32);
        let _ = (a.process(x(i)), b.process(x(i)));
    }
    let mut diff = Vec::new();
    for i in settle..settle + FS as usize / 5 {
        a.set_distance(10.0, FS as f32);
        b.set_distance(30.0, FS as f32);
        diff.push((b.process(x(i)) - a.process(x(i))).abs());
    }
    let first = diff[..5].iter().fold(0.0f32, |m, v| m.max(*v));
    let steady = diff[diff.len() - 480..].iter().fold(0.0f32, |m, v| m.max(*v));
    assert!(steady > 0.05, "30m와 10m의 12kHz 차이가 너무 작다({steady})");
    assert!(
        first < 0.15 * steady,
        "거리를 바꾼 직후 5샘플 안에 감쇠 변화의 {:.0}%가 이미 걸렸다(계단)",
        100.0 * first / steady
    );
}

/// 공기 흡음 필터 단독으로 `dist_m` 거리의 `freq` 사인 크기(RMS).
fn air_model_level(dist_m: f32, freq: f32) -> f32 {
    use rust_lib_atmos_mixer_pro::audio::dsp::acoustic_physics::AirAbsorptionFilter;
    let mut f = AirAbsorptionFilter::new();
    let (mut acc, mut n) = (0.0f64, 0usize);
    for i in 0..(FS as usize) {
        f.set_distance(dist_m, FS as f32);
        let y = f.process((std::f32::consts::TAU * freq * i as f32 / FS as f32).sin());
        if i > FS as usize / 2 {
            acc += (y as f64).powi(2);
            n += 1;
        }
    }
    (acc / n as f64).sqrt() as f32
}

/// 청취 지점에서 수평 방향 (dx, dy)로 2.83m, 높이 `elevation_deg`인 스피커.
fn speaker_at_elevation(elevation_deg: f32) -> Point3D {
    let (dx, dy) = (-2.0f32, 2.0f32);
    let horiz = (dx * dx + dy * dy).sqrt();
    let dz = horiz * elevation_deg.to_radians().tan();
    p(LISTENER.0 + dx, LISTENER.1 + dy, LISTENER.2 + dz)
}

#[test]
fn 높이_매단_스피커는_고도각_hrtf로_렌더링된다() {
    // 옆쪽(45°) 스피커는 귀 높이면 두 귀 크기 차(ILD)가 크고, 머리 위로 올라갈수록 작아진다.
    // 예전에는 높이를 무시해 천장 가까이 매단 스피커도 귀 높이처럼 들렸다.
    let ild = |elevation: f32| {
        let (l, r) = headphone_lr(&mut mixer_with_speaker(speaker_at_elevation(elevation), true), 3000.0, 30);
        db(l / r).abs()
    };
    let ear = ild(0.0);
    let overhead = ild(80.0);
    assert!(ear > 6.0, "귀 높이 45° 스피커의 ILD가 {ear:.1}dB로 너무 작다");
    assert!(
        overhead < ear * 0.5,
        "80° 위 스피커의 ILD {overhead:.1}dB가 귀 높이 {ear:.1}dB와 비슷하다 — 높이가 반영되지 않았다"
    );
}

#[test]
fn 높이만_바뀌어도_hrir을_다시_만든다() {
    let mut m = mixer_with_speaker(speaker_at_elevation(0.0), true);
    headphone_lr(&mut m, 1000.0, 2);
    let before = m.binaural.debug_hrir_rebuild_count();
    m.channel_positions[SPK] = Some(speaker_at_elevation(40.0));
    m.recalculate_binaural_channel_azimuths();
    headphone_lr(&mut m, 1000.0, 2);
    assert!(
        m.binaural.debug_hrir_rebuild_count() > before,
        "방위각은 같고 높이만 40° 바뀌었는데 HRIR을 다시 만들지 않았다"
    );
}

#[test]
fn 고도_링_조회는_전수_조사와_같은_최근접_측정점을_고른다() {
    let m = mixer_with_speaker(speaker_at_elevation(0.0), true);
    let mut checked = 0;
    for el in (-40..=90).step_by(7) {
        for az in (-180..180).step_by(11) {
            let fast = m.binaural.debug_nearest_hrirs(az as f32, el as f32);
            let slow = m.binaural.debug_nearest_hrirs_brute_force(az as f32, el as f32);
            // 가장 가까운 측정점은 반드시 같아야 하고, 세 점의 거리 합도 같아야 한다.
            assert_eq!(fast[0].0, slow[0].0, "az {az} el {el}: 최근접 측정점이 다르다");
            let sum = |x: [(usize, f32); 3]| x.iter().map(|v| v.1).sum::<f32>();
            assert!((sum(fast) - sum(slow)).abs() < 1e-3, "az {az} el {el}: 세 최근접 점이 다르다");
            checked += 1;
        }
    }
    assert!(checked > 300);
}

fn low_shelf(gain: f32) -> [EqBand; MAX_SIM_BANDS] {
    let mut bands: [EqBand; MAX_SIM_BANDS] = std::array::from_fn(|_| EqBand::default());
    bands[0] = EqBand {
        enabled: true,
        freq: 300.0,
        gain,
        q_factor: 0.707,
        filter_type: EqType::LowShelf,
        slope_db_per_oct: 12,
    };
    bands
}

#[test]
fn 현장_물리_밴드는_헤드폰에만_걸린다() {
    let front = p(LISTENER.0, LISTENER.1 + 3.0, LISTENER.2);
    let level = |gain: f32, freq: f32| {
        let mut m = mixer_with_speaker(front.clone(), true);
        m.binaural.set_channel_sim_bands(SPK, &low_shelf(gain));
        headphone_level(&mut m, freq)
    };
    let boost = db(level(6.0, 50.0) / level(0.0, 50.0));
    assert!((boost - 6.0).abs() < 0.5, "헤드폰 50Hz가 +6dB 로우셸프에 {boost:+.2}dB 반응");
    let mid = db(level(6.0, 3000.0) / level(0.0, 3000.0));
    assert!(mid.abs() < 0.3, "로우셸프가 3kHz를 {mid:+.2}dB 바꿨다");

    // 실제 스피커로 나가는 채널(CH3 이후)은 그대로다.
    let mut m = mixer_with_speaker(front.clone(), true);
    m.binaural.set_channel_sim_bands(SPK, &low_shelf(6.0));
    let mut buf = vec![0.0f32; CH * BLOCK];
    for f in 0..BLOCK {
        buf[f * CH + SPK] = 0.2 * (f as f32 * 0.01).sin();
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
fn 현장_물리_밴드가_바뀌어도_딸깍_소리가_나지_않는다() {
    let mut m = mixer_with_speaker(p(LISTENER.0, LISTENER.1 + 3.0, LISTENER.2), true);
    let mut n = 0usize;
    let mut run = |m: &mut AudioMixer, blocks: usize| {
        let mut out = Vec::new();
        for _ in 0..blocks {
            let mut buf = vec![0.0f32; CH * BLOCK];
            for f in 0..BLOCK {
                buf[f * CH + SPK] = 0.3 * (std::f32::consts::TAU * 120.0 * n as f32 / FS as f32).sin();
                n += 1;
            }
            m.binaural.process_interleaved(&mut buf, CH);
            for f in 0..BLOCK {
                out.push(buf[f * CH]);
            }
        }
        out
    };
    run(&mut m, 30);
    let before = run(&mut m, 20);
    m.binaural.set_channel_sim_bands(SPK, &low_shelf(9.0));
    let transition = run(&mut m, 20);
    let after = run(&mut m, 20);
    let max_step = |x: &[f32]| x.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max);
    let reference = max_step(&before).max(max_step(&after));
    let tr = max_step(&transition);
    assert!(tr <= reference * 1.5, "물리 밴드 변경 순간 불연속: 전환 {tr:.5}, 정상 {reference:.5}");
}
