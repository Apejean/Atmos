//! 합성 반사음은 실제 방의 반사처럼 **고역이 죽어서** 돌아와야 한다.
//!
//! 지금 모델은 이상적인 거울면을 가정한다. 그래서 반사음이 직접음의 고역을 그대로
//! 되돌려주고, 직접음과 겹치면서 4~8kHz에 +5~7dB 피크(콤필터)를 만들었다
//! (실기 보고: "바이노럴 켜면 고역이 세게 들어온다").
//!
//! 실제로 고역을 잃는 이유 셋을 반사음마다 반영한다:
//! 1. **스피커 방향성** — 벽·천장으로 나가는 고역은 정면보다 훨씬 약하다.
//! 2. **재질의 고역 흡음** — 흡음률은 고역에서 더 높다.
//! 3. **표면 산란** — 실제 면은 완전한 거울이 아니라 고역을 흩뿌린다.

use rust_lib_atmos_mixer_pro::audio::acoustic::compute_early_reflection_taps;
use rust_lib_atmos_mixer_pro::audio::dsp::dsp_utils::ChannelDspState;
use rust_lib_atmos_mixer_pro::common::config::{Point3D, RoomZone};

const FS: f32 = 48000.0;

fn aimed(x: f32, y: f32, z: f32, yaw: f32, pitch: f32) -> Point3D {
    Point3D {
        x,
        y,
        z,
        yaw_rotation: yaw,
        pitch_tilt: pitch,
        dispersion_angle: 90.0,
        ..Default::default()
    }
}

/// 15m x 20m x 15m 콘크리트 홀(사용자 실제 방). 귀 높이 1.6m.
fn hall(absorption: f32) -> RoomZone {
    RoomZone {
        boundary_min: Point3D { x: 0.0, y: 0.0, z: 0.0, ..Default::default() },
        boundary_max: Point3D { x: 15.0, y: 20.0, z: 15.0, ..Default::default() },
        absorption_coeff: absorption,
        ear_level: 1.6,
        ..Default::default()
    }
}

#[test]
fn 등을_돌린_면의_반사는_고역이_크게_깎인다() {
    // 스피커를 뒤쪽 벽(+Y) 근처에 두고 청취자(−Y 방향)를 향해 조준한다.
    // 뒤쪽 +Y벽 반사는 스피커 등 뒤로 나가야 하므로 고역이 거의 없어야 한다.
    let zone = hall(0.02);
    let speaker = aimed(7.5, 18.0, 2.0, 180.0, 0.0);
    let taps = compute_early_reflection_taps(&speaker, &zone);

    let back = taps[5]; // +Y벽 (스피커 뒤)
    let front = taps[4]; // -Y벽 (청취자 너머, 조준 방향)

    assert!(
        back.hf_shelf_db <= -12.0,
        "등 뒤 벽 반사의 고역이 충분히 깎이지 않았다: {:.1}dB",
        back.hf_shelf_db
    );
    assert!(
        front.hf_shelf_db > back.hf_shelf_db + 6.0,
        "조준 방향 면과 등 뒤 면의 고역 감쇠가 비슷하다(방향성이 반영되지 않았다): \
         앞 {:.1}dB, 뒤 {:.1}dB",
        front.hf_shelf_db,
        back.hf_shelf_db
    );
}

#[test]
fn 분산각_안의_반사는_방향성_감쇠를_받지_않는다() {
    let zone = hall(0.02);
    let speaker = aimed(7.5, 18.0, 2.0, 180.0, 0.0);
    let taps = compute_early_reflection_taps(&speaker, &zone);
    let front = taps[4]; // 조준 방향(-Y벽)

    // 재질·산란·공기 흡음만 남아야 한다. 방향성까지 겹치면 -10dB보다 커진다.
    assert!(
        front.hf_shelf_db > -8.0 && front.hf_shelf_db < 0.0,
        "콘 안쪽 반사의 고역 감쇠가 과하다: {:.1}dB",
        front.hf_shelf_db
    );
}

#[test]
fn 재질_흡음이_크면_고역이_더_깎인다() {
    let speaker = aimed(7.5, 18.0, 2.0, 180.0, 0.0);
    let live = compute_early_reflection_taps(&speaker, &hall(0.05));
    let dead = compute_early_reflection_taps(&speaker, &hall(0.6));

    assert!(
        dead[4].hf_shelf_db < live[4].hf_shelf_db - 1.0,
        "흡음이 큰 방에서 고역이 더 깎이지 않았다: live {:.1}dB, dead {:.1}dB",
        live[4].hf_shelf_db,
        dead[4].hf_shelf_db
    );
}

/// 지정한 탭 하나만 심고 정상 상태로 맞춘 DSP 상태.
fn one_tap(delay_ms: f32, gain: f32, hf_db: f32) -> ChannelDspState {
    let mut st = ChannelDspState::default();
    st.taps[0].target_delay_ms = delay_ms;
    st.taps[0].current_delay_ms = delay_ms;
    st.taps[0].target_gain = gain;
    st.taps[0].current_gain = gain;
    // 고역 셸프는 목표만 준다. 실제 엔진처럼 계수가 조금씩 따라가며 수렴한다(Law 3).
    st.taps[0].target_hf_shelf_db = hf_db;
    st.target_early_ref_mix = 1.0;
    st.current_early_ref_mix = 1.0;
    st
}

fn added_level(freq: f32, hf_db: f32) -> f32 {
    let rms = |st: &mut ChannelDspState| {
        let n = (0.3 * FS) as usize;
        let (mut sum, mut cnt) = (0.0f32, 0.0f32);
        for i in 0..n {
            let x = (std::f32::consts::TAU * freq * i as f32 / FS).sin();
            let y = st.process(x, FS);
            if i > n * 2 / 3 {
                sum += y * y;
                cnt += 1.0;
            }
        }
        (sum / cnt).sqrt()
    };
    // 탭이 더한 성분만 보기 위해 게인 0(=탭 없음)과 비교한다.
    let wet = rms(&mut one_tap(10.0, 0.5, hf_db));
    let dry = rms(&mut one_tap(10.0, 0.0, hf_db));
    20.0 * (wet / dry).log10()
}

#[test]
fn 고역_감쇠가_반사음_출력에_실제로_적용된다() {
    // 저역은 그대로, 고역만 줄어야 한다.
    let low_damped = added_level(200.0, -12.0);
    let low_flat = added_level(200.0, 0.0);
    let high_damped = added_level(6000.0, -12.0);
    let high_flat = added_level(6000.0, 0.0);

    assert!(
        (low_damped - low_flat).abs() < 0.5,
        "저역이 함께 깎였다: 감쇠 {low_damped:.2}dB vs 평탄 {low_flat:.2}dB"
    );
    // 직접음과의 합산(콤)이라 반사음 -12dB가 곧 총합 -12dB는 아니다.
    // 반사음 0.5 -> 0.125이면 합산 피크는 +3.5dB -> +1.0dB가 된다.
    assert!(
        high_flat - high_damped > 2.0,
        "고역이 줄지 않았다: 감쇠 {high_damped:.2}dB vs 평탄 {high_flat:.2}dB"
    );
}

/// 실제 저장 레이아웃(천장 근처 14.75m에 매달아 청취자를 향해 조준한 ch3).
fn ch3() -> Point3D {
    // 청취자(7.5, 10, 1.6) 방향으로 조준: 수평 방위 + 아래로 기운 각.
    let (dx, dy, dz): (f32, f32, f32) = (7.5 - 14.8, 10.0 - 4.15, 1.6 - 14.75);
    let yaw = dx.atan2(dy).to_degrees();
    let pitch = dz.atan2((dx * dx + dy * dy).sqrt()).to_degrees();
    aimed(14.8, 4.15, 14.75, yaw, pitch)
}

fn band_response_db(freq: f32, damping: bool) -> f32 {
    let zone = hall(0.018333);
    let taps = compute_early_reflection_taps(&ch3(), &zone);
    let make = |on: bool| {
        let mut st = ChannelDspState::default();
        for (i, t) in taps.iter().enumerate() {
            st.taps[i].target_delay_ms = t.delay_ms;
            st.taps[i].current_delay_ms = t.delay_ms;
            st.taps[i].target_gain = if on { t.gain } else { 0.0 };
            st.taps[i].current_gain = st.taps[i].target_gain;
            st.taps[i].target_hf_shelf_db = if damping { t.hf_shelf_db } else { 0.0 };
        }
        st.target_early_ref_mix = 1.0;
        st.current_early_ref_mix = 1.0;
        st
    };
    let rms = |st: &mut ChannelDspState| {
        let n = (0.3 * FS) as usize;
        let (mut sum, mut cnt) = (0.0f32, 0.0f32);
        for i in 0..n {
            let x = (std::f32::consts::TAU * freq * i as f32 / FS).sin();
            let y = st.process(x, FS);
            if i > n * 2 / 3 {
                sum += y * y;
                cnt += 1.0;
            }
        }
        (sum / cnt).sqrt()
    };
    let wet = rms(&mut make(true));
    let dry = rms(&mut make(false));
    20.0 * (wet / dry).log10()
}

#[test]
fn 실제_레이아웃의_고역_출렁임이_줄어든다() {
    // 반사가 직접음과 겹치며 만드는 콤 출렁임(피크와 골 모두)이 고역에서 작아져야 한다.
    // 8kHz처럼 원래 골인 지점도 얕아지는 게 정상이다 — 반사가 작아지면 양쪽 다 0dB로 모인다.
    for freq in [4000.0, 8000.0] {
        let flat = band_response_db(freq, false);
        let damped = band_response_db(freq, true);
        println!("{freq:.0}Hz: 감쇠 없음 {flat:+.1}dB -> 감쇠 적용 {damped:+.1}dB");
        assert!(
            flat.abs() - damped.abs() >= 2.0,
            "{freq:.0}Hz 출렁임이 충분히 줄지 않았다: 감쇠 없음 {flat:+.1}dB -> 감쇠 적용 {damped:+.1}dB"
        );
    }
    // 저역(공간감)은 남아 있어야 한다 — 전체를 깎아버린 게 아님을 확인.
    let low = band_response_db(125.0, true);
    assert!(low.abs() > 1.0, "저역 반사까지 사라졌다: {low:+.1}dB");
}

#[test]
fn 고역_감쇠_변경은_딸깍_소리를_내지_않는다() {
    // 스피커를 드래그하면 반사 각도가 계속 바뀐다(Law 3: 즉시 스냅 금지).
    let mut st = one_tap(10.0, 0.6, 0.0);
    let mut max_step_steady = 0.0f32;
    let mut prev = 0.0f32;
    let tone = |i: usize| (std::f32::consts::TAU * 3000.0 * i as f32 / FS).sin();

    for i in 0..4800 {
        let y = st.process(tone(i), FS);
        if i > 2400 {
            max_step_steady = max_step_steady.max((y - prev).abs());
        }
        prev = y;
    }
    st.taps[0].target_hf_shelf_db = -18.0; // 급격한 각도 변화
    let mut max_step_transition = 0.0f32;
    for i in 4800..9600 {
        let y = st.process(tone(i), FS);
        max_step_transition = max_step_transition.max((y - prev).abs());
        prev = y;
    }

    assert!(
        max_step_transition <= max_step_steady * 1.5,
        "고역 감쇠가 튀었다: 전환 {max_step_transition:.5}, 정상 {max_step_steady:.5}"
    );
}
