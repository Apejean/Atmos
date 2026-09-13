//! 초기반사음이 **룸 형상에서 실제 출력까지** 이어지는지 검증하는 종단 테스트.
//!
//! dsp.rs의 기존 단위 테스트는 탭을 손으로 심어 DSP 블록만 확인한다.
//! 이 테스트는 그 앞단을 잇는다: RoomZone 형상 -> compute_early_reflection_taps
//! -> (엔진 핸들러가 하는 것과 같은) 탭 이관 -> ChannelDspState::process 출력.
//!
//! 이렇게 해야 "기하는 맞는데 소리에 안 실린다" 또는 "소리는 나는데 지연이
//! 물리와 무관하다" 같은 배관 결함을 잡을 수 있다.

use rust_lib_atmos_mixer_pro::audio::acoustic::{
    calculate_acoustic_delay_ms, compute_early_reflection_taps, distance_3d,
    MAX_EARLY_REFLECTION_TAPS,
};
use rust_lib_atmos_mixer_pro::audio::dsp::dsp_utils::ChannelDspState;
use rust_lib_atmos_mixer_pro::common::config::{Point3D, RoomZone};

const FS: f32 = 48000.0;

fn point(x: f32, y: f32, z: f32) -> Point3D {
    Point3D { x, y, z, ..Default::default() }
}

/// 10m(W) x 8m(D) x 3m(H) 슈박스 룸. 귀 높이 1.2m, 흡음 0.2.
fn room() -> RoomZone {
    RoomZone {
        boundary_min: point(0.0, 0.0, 0.0),
        boundary_max: point(10.0, 8.0, 3.0),
        absorption_coeff: 0.2,
        ear_level: 1.2,
        ..Default::default()
    }
}

/// 엔진의 `UpdateSpatialConfig` 핸들러가 하는 이관을 그대로 재현한다
/// (engine.rs: taps[i].target_delay_ms / target_gain 대입).
/// 스무딩 수렴을 기다리지 않도록 current도 같이 스냅한다.
fn install_taps(state: &mut ChannelDspState, speaker: &Point3D, zone: &RoomZone, mix: f32) {
    let taps = compute_early_reflection_taps(speaker, zone);
    for i in 0..MAX_EARLY_REFLECTION_TAPS {
        state.taps[i].target_delay_ms = taps[i].delay_ms;
        state.taps[i].current_delay_ms = taps[i].delay_ms;
        state.taps[i].target_gain = taps[i].gain;
        state.taps[i].current_gain = taps[i].gain;
    }
    state.target_early_ref_mix = mix;
    state.current_early_ref_mix = mix;
}

/// 임펄스를 넣고 n개 샘플을 받아온다.
fn impulse_response(state: &mut ChannelDspState, n: usize) -> Vec<f32> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let x = if i == 0 { 1.0 } else { 0.0 };
        out.push(state.process(x, FS));
    }
    out
}

#[test]
fn room_geometry_produces_reflections_at_physically_correct_delays() {
    let zone = room();
    let speaker = point(2.0, 2.0, 2.5);

    // 차분 측정: 같은 상태에서 mix만 1.0과 0.0으로 두고 뺀다.
    //
    // 절대값으로 "그 구간에 에너지가 있는가"를 보면 안 된다. process()에는
    // 게인 스무딩 등에서 오는 아주 작은 꼬리가 전 구간에 깔려 있어서, 초기반사음을
    // 아예 더하지 않아도 임계값을 넘어버린다(이 테스트의 첫 버전이 그래서 무효였다).
    // 초기반사음이 만든 차이만 분리해야 한다.
    let mut wet = ChannelDspState::new();
    install_taps(&mut wet, &speaker, &zone, 1.0);

    let mut dry = ChannelDspState::new();
    install_taps(&mut dry, &speaker, &zone, 0.0);

    let ir_wet = impulse_response(&mut wet, 4096);
    let ir_dry = impulse_response(&mut dry, 4096);

    let diff: Vec<f32> = ir_wet
        .iter()
        .zip(ir_dry.iter())
        .map(|(w, d)| w - d)
        .collect();

    let taps = compute_early_reflection_taps(&speaker, &zone);

    // 기대 지연을 기하로 독립 계산해 구현과 대조한다.
    let listener = point(5.0, 4.0, zone.ear_level);
    let floor_image = point(speaker.x, speaker.y, -speaker.z);
    let ceiling_image = point(speaker.x, speaker.y, 2.0 * 3.0 - speaker.z);

    for (label, image, tap_idx) in [
        ("바닥", floor_image, 0usize),
        ("천장", ceiling_image, 1usize),
    ] {
        let path = distance_3d(&image, &listener);
        let delay_ms = calculate_acoustic_delay_ms(path);

        // 구현이 계산한 탭과 독립 계산이 일치하는지 먼저 확인.
        assert!(
            (taps[tap_idx].delay_ms - delay_ms).abs() < 1e-3,
            "{} 탭 지연이 기하 계산과 다르다. 구현 {:.3}ms, 기대 {:.3}ms",
            label, taps[tap_idx].delay_ms, delay_ms
        );

        let idx = (delay_ms / 1000.0 * FS) as usize;
        assert!(idx + 2 < diff.len(), "{} 탭이 측정 구간을 벗어났다", label);

        // 임펄스가 그 지연 위치에 탭 게인만큼의 사본으로 나타나야 한다.
        let peak = diff[idx.saturating_sub(1)..=idx + 1]
            .iter()
            .fold(0.0f32, |m, v| m.max(v.abs()));

        assert!(
            (peak - taps[tap_idx].gain).abs() < 0.02,
            "{} 반사의 진폭이 탭 게인과 다르다. 실측 {:.4}, 기대 {:.4} (지연 {:.2}ms, 인덱스 {})",
            label, peak, taps[tap_idx].gain, delay_ms, idx
        );
    }

    // 탭이 없는 조용한 구간에서는 차이가 사실상 0이어야 한다
    // (초기반사음이 전 구간에 번지고 있지 않은지 확인).
    let quiet = &diff[50..500];
    let quiet_max = quiet.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    assert!(
        quiet_max < 1e-5,
        "탭이 없는 구간에 초기반사음이 새고 있다. 최대 {:e}",
        quiet_max
    );
}

#[test]
fn mix_zero_leaves_signal_untouched_even_with_real_room_geometry() {
    let zone = room();
    let speaker = point(2.0, 2.0, 2.5);

    let mut with_mix = ChannelDspState::new();
    install_taps(&mut with_mix, &speaker, &zone, 0.0);

    let mut bypass = ChannelDspState::new();
    // 탭을 아예 심지 않은 상태(기본 mix 0.0)와 비교한다.

    let a = impulse_response(&mut with_mix, 4096);
    let b = impulse_response(&mut bypass, 4096);

    assert_eq!(
        a, b,
        "mix=0인데 출력이 달라졌다. 초기반사음이 mix와 무관하게 새고 있다."
    );
}

#[test]
fn larger_room_pushes_reflections_later_and_quieter() {
    // 방이 커지면 경로가 길어져 지연은 늘고 게인(1/거리)은 줄어야 한다.
    let small = RoomZone {
        boundary_min: point(0.0, 0.0, 0.0),
        boundary_max: point(6.0, 5.0, 3.0),
        absorption_coeff: 0.2,
        ear_level: 1.2,
        ..Default::default()
    };
    let large = RoomZone {
        boundary_min: point(0.0, 0.0, 0.0),
        boundary_max: point(30.0, 25.0, 3.0),
        absorption_coeff: 0.2,
        ear_level: 1.2,
        ..Default::default()
    };

    // 각 방의 같은 상대 위치(중앙에서 X로 조금 치우친 곳)에 스피커를 둔다.
    let taps_small = compute_early_reflection_taps(&point(1.0, 2.5, 2.0), &small);
    let taps_large = compute_early_reflection_taps(&point(5.0, 12.5, 2.0), &large);

    // -X벽(인덱스 2) 반사를 비교한다.
    let s = taps_small[2];
    let l = taps_large[2];

    assert!(
        l.delay_ms > s.delay_ms,
        "큰 방의 반사 지연이 더 길어야 한다. 작은방 {:.2}ms, 큰방 {:.2}ms",
        s.delay_ms, l.delay_ms
    );
    assert!(
        l.gain < s.gain,
        "큰 방의 반사 게인이 더 작아야 한다(1/거리 감쇠). 작은방 {:.4}, 큰방 {:.4}",
        s.gain, l.gain
    );
}

#[test]
fn absorption_reduces_reflection_gain() {
    let speaker = point(2.0, 2.0, 2.5);

    let live = RoomZone { absorption_coeff: 0.05, ..room() };
    let dead = RoomZone { absorption_coeff: 0.9, ..room() };

    let t_live = compute_early_reflection_taps(&speaker, &live);
    let t_dead = compute_early_reflection_taps(&speaker, &dead);

    for i in 0..MAX_EARLY_REFLECTION_TAPS {
        assert!(
            t_dead[i].gain < t_live[i].gain,
            "흡음이 큰 방의 탭 {} 게인이 더 작아야 한다. live {:.4}, dead {:.4}",
            i, t_live[i].gain, t_dead[i].gain
        );
        // 지연은 흡음과 무관하게 같아야 한다(기하만의 함수).
        assert!(
            (t_dead[i].delay_ms - t_live[i].delay_ms).abs() < 1e-6,
            "흡음이 지연을 바꿨다. 탭 {}", i
        );
    }
}
