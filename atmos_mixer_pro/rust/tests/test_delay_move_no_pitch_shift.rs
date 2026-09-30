//! 딜레이 탭 이동이 피치를 휘게 하지 않는지 검증한다.
//!
//! 실기 보고: "소리를 재생시키고 스피커를 움직이면 테이프 스톱/스타트 같은
//! 소리가 난다." 원인은 딜레이 라인의 읽기 위치를 연속적으로 끌고 간 것이다.
//! 읽기 위치가 움직이면 재생 속도가 바뀌는 것과 같아서 피치가 휜다(도플러).
//! 움직이는 음원에는 맞는 동작이지만, 정지한 스피커의 시간정렬 보정값을
//! 바꾸는 데는 맞지 않는다.
//!
//! 지금은 이전 탭과 새 탭에서 각각 고정 위치로 읽어 짧게 교차 페이드한다.
//! 읽기 위치가 고정이므로 피치가 변하지 않는다.

use rust_lib_atmos_mixer_pro::audio::dsp::dsp_utils::ChannelDspState;

const FS: f32 = 48000.0;
const FREQ: f32 = 1000.0;

/// 사인파를 흘리면서 도중에 딜레이 목표를 크게 바꾸고, 구간별로 영점 교차
/// 횟수를 센다. 피치가 휘면 교차 횟수가 눈에 띄게 달라진다.
fn zero_crossings_after_delay_jump(from_ms: f32, to_ms: f32) -> (usize, usize) {
    let mut st = ChannelDspState::new();
    st.update_delay_target(from_ms);

    // 딜레이가 자리잡도록 충분히 흘린다.
    for i in 0..(FS as usize) {
        let t = i as f32 / FS;
        st.process((2.0 * std::f32::consts::PI * FREQ * t).sin(), FS);
    }

    // 기준 구간: 목표를 바꾸기 직전 0.2초.
    let mut baseline = 0usize;
    let mut prev = 0.0f32;
    for i in 0..(FS as usize / 5) {
        let t = (FS as usize + i) as f32 / FS;
        let out = st.process((2.0 * std::f32::consts::PI * FREQ * t).sin(), FS);
        if prev <= 0.0 && out > 0.0 {
            baseline += 1;
        }
        prev = out;
    }

    // 목표를 크게 바꾼 직후 0.2초.
    st.update_delay_target(to_ms);
    let mut after = 0usize;
    prev = 0.0;
    for i in 0..(FS as usize / 5) {
        let t = (FS as usize + FS as usize / 5 + i) as f32 / FS;
        let out = st.process((2.0 * std::f32::consts::PI * FREQ * t).sin(), FS);
        if prev <= 0.0 && out > 0.0 {
            after += 1;
        }
        prev = out;
    }

    (baseline, after)
}

#[test]
fn moving_the_delay_tap_does_not_bend_pitch() {
    // 방을 가로지르는 수준의 변화(2ms -> 40ms).
    let (baseline, after) = zero_crossings_after_delay_jump(2.0, 40.0);

    // 1kHz를 0.2초 재생하면 약 200회 교차한다. 피치가 휘면 이 수가 달라진다.
    let diff = (baseline as i64 - after as i64).abs();
    assert!(
        diff <= 2,
        "딜레이를 옮기는 동안 피치가 변했다(테이프 스톱 현상). \
         기준 구간 {}회, 변경 직후 {}회 (차이 {})",
        baseline, after, diff
    );
}

#[test]
fn delay_tap_move_keeps_signal_audible() {
    let mut st = ChannelDspState::new();
    st.update_delay_target(2.0);
    for i in 0..(FS as usize) {
        let t = i as f32 / FS;
        st.process((2.0 * std::f32::consts::PI * FREQ * t).sin(), FS);
    }

    // 이동 중에도 소리가 끊기면 안 된다(등출력 페이드).
    st.update_delay_target(40.0);
    let mut min_rms = f32::MAX;
    let window = 256;
    let mut acc = 0.0f32;
    let mut n = 0usize;
    for i in 0..(FS as usize / 10) {
        let t = (FS as usize + i) as f32 / FS;
        let out = st.process((2.0 * std::f32::consts::PI * FREQ * t).sin(), FS);
        acc += out * out;
        n += 1;
        if n == window {
            let rms = (acc / window as f32).sqrt();
            if rms < min_rms {
                min_rms = rms;
            }
            acc = 0.0;
            n = 0;
        }
    }

    assert!(
        min_rms > 0.2,
        "딜레이 이동 중 소리가 크게 꺼졌다(교차 페이드가 음량을 유지하지 못함). \
         최소 RMS={:.4}",
        min_rms
    );
}
