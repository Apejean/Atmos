//! 위상 반전은 자동으로 걸린다. 청취자 뒤쪽에 놓인 스피커는 전면 스피커와의
//! 저역 상쇄를 막으려고 180도 뒤집는데, 스피커를 드래그해서 그 경계를
//! 넘나들면 반전이 계속 토글된다.
//!
//! `out *= -1.0`으로 즉시 뒤집으면 파형 부호가 한 샘플 만에 반대가 되어
//! 최대 진폭의 계단이 생긴다 — 확실하게 "딱" 하고 들린다(DSP Law 3).
//!
//! 실기 보고: "스피커를 옮길 때마다 딱딱거리는 소리가 난다".

use rust_lib_atmos_mixer_pro::audio::dsp::dsp_utils::ChannelDspState;

const FS: f32 = 48_000.0;

/// 1kHz 사인을 n샘플 흘리면서 인접 샘플 간 최대 변화량을 잰다.
///
/// `prev`를 호출자가 들고 있어야 한다. 창 안에서만 비교하면 창 경계에 걸린
/// 튐을 통째로 놓친다 — 처음 작성한 버전이 실제로 그랬고, 옛 코드(즉시 반전)
/// 에서도 테스트가 통과해 버렸다. 반전은 하필 창의 0번 샘플에서 일어난다.
fn run(dsp: &mut ChannelDspState, phase: &mut f32, prev: &mut f32, n: usize) -> f32 {
    let mut max_jump = 0.0f32;
    for _ in 0..n {
        let x = (*phase).sin();
        *phase += 2.0 * std::f32::consts::PI * 1000.0 / FS;
        let y = dsp.process(x, FS);
        let jump = (y - *prev).abs();
        if jump > max_jump {
            max_jump = jump;
        }
        *prev = y;
    }
    max_jump
}

#[test]
fn polarity_flip_does_not_step_the_waveform() {
    let mut dsp = ChannelDspState::new();
    let mut phase = 0.0f32;
    let mut prev = 0.0f32;

    // 정상 상태의 자연스러운 샘플 간 변화량(1kHz 사인의 기울기).
    for _ in 0..8 {
        run(&mut dsp, &mut phase, &mut prev, 1024);
    }
    let baseline = run(&mut dsp, &mut phase, &mut prev, 1024);
    assert!(baseline > 0.0, "기준 신호가 흐르지 않는다");

    // 반전 시점을 사인의 **마루**에 맞춘다.
    //
    // 워밍업 9216샘플은 1kHz/48kHz 기준으로 정확히 192주기라, 그대로 뒤집으면
    // 하필 영교차점에서 반전이 일어나 부호를 바꿔도 값이 0이라 튀지 않는다.
    // 그 상태로는 옛 코드(즉시 반전)에서도 테스트가 통과해 버렸다.
    // 1/4주기(12샘플)만 더 흘려서 최악의 조건에서 재도록 한다.
    run(&mut dsp, &mut phase, &mut prev, 12);

    // 스피커가 청취자 뒤쪽 경계를 넘어간 상황.
    dsp.phase_invert = true;

    let mut worst = 0.0f32;
    for _ in 0..8 {
        let jump = run(&mut dsp, &mut phase, &mut prev, 1024);
        if jump > worst {
            worst = jump;
        }
    }

    // 즉시 뒤집으면 한 샘플 만에 2배 진폭의 계단이 생겨 기준선을 훨씬 넘는다.
    assert!(
        worst < baseline * 2.0,
        "위상 반전이 파형에 계단을 만든다(딸깍 소리): baseline={baseline:.6}, worst={worst:.6}"
    );

    // 실제로 반전이 끝까지 갔는지 확인(가다 말면 레벨만 줄어든 채로 남는다).
    assert!(
        (dsp.current_polarity + 1.0).abs() < 1e-3,
        "위상 반전이 완료되지 않았다: {}",
        dsp.current_polarity
    );
}
