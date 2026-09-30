//! 스피커를 움직이면 자동 EQ가 연속적으로 바뀐다. 그때 바이쿼드 계수를
//! 즉시 갈아끼우면 파형이 한 샘플 만에 튀어서 "딱" 소리가 난다(DSP Law 3).
//!
//! 실기 보고: "스피커를 옮길 때마다 딱딱거리는 소리가 난다".

use rust_lib_atmos_mixer_pro::audio::dsp::dsp_utils::ChannelDspState;
use rust_lib_atmos_mixer_pro::common::config::{EqBand, EqType};

const FS: f32 = 48_000.0;
const BLOCK: usize = 1024;
const TONE_HZ: f32 = 1000.0;

fn band(freq: f32, gain: f32) -> EqBand {
    EqBand {
        enabled: true,
        freq,
        gain,
        q_factor: 4.0,
        filter_type: EqType::Bell,
        slope_db_per_oct: 12,
    }
}

/// 짧은 창(128샘플 = 2.7ms) 단위로 출력 RMS를 잰다.
///
/// 클릭을 재는 지표로 "파형의 불연속"을 쓰면 안 된다. 바이쿼드 계수를
/// 통째로 갈아끼워도 필터 상태는 이어지므로 출력은 수학적으로 연속이다.
/// 실제로 들리는 건 **레벨이 순식간에 꺾이는 것**이다. 그래서 짧은 창의
/// 레벨 궤적을 보고, 변화가 한 창에 몰렸는지 여러 창에 퍼졌는지를 본다.
fn window_rms(dsp: &mut ChannelDspState, phase: &mut f32, n: usize) -> f32 {
    let mut sum_sq = 0.0f32;
    for _ in 0..n {
        let x = (*phase).sin();
        *phase += 2.0 * std::f32::consts::PI * TONE_HZ / FS;
        let y = dsp.process(x, FS);
        sum_sq += y * y;
    }
    (sum_sq / n as f32).sqrt()
}

#[test]
fn eq_change_is_spread_over_time_not_dumped_into_one_window() {
    const WIN: usize = 128; // 2.7ms
    let mut dsp = ChannelDspState::new();
    let mut phase = 0.0f32;

    // 테스트 톤과 **같은 주파수**의 벨을 쓴다. 톤에서 멀리 떨어진 밴드를
    // 움직이면 출력이 거의 안 변해서, 계수를 스냅해도 테스트가 통과해
    // 버린다(처음 작성했던 버전이 실제로 그랬다).
    dsp.update_eq_targets(&vec![band(TONE_HZ, 0.0)], FS);
    for _ in 0..400 {
        window_rms(&mut dsp, &mut phase, WIN);
    }
    let before = window_rms(&mut dsp, &mut phase, WIN);
    assert!(before > 0.1, "기준 신호가 흐르지 않는다: {before}");

    // 스피커를 크게 움직였을 때에 해당하는 급격한 EQ 변경: 1kHz를 -18dB.
    dsp.update_eq_targets(&vec![band(TONE_HZ, -18.0)], FS);

    let mut worst_step_db = 0.0f32;
    let mut prev = before;
    let mut last = before;
    for _ in 0..240 {
        let r = window_rms(&mut dsp, &mut phase, WIN);
        let step = 20.0 * (r.max(1e-9) / prev.max(1e-9)).log10();
        if step.abs() > worst_step_db {
            worst_step_db = step.abs();
        }
        prev = r;
        last = r;
    }

    // 실제로 -18dB까지 내려갔는지 먼저 확인(변화가 없으면 테스트가 무의미).
    let total_db = 20.0 * (last.max(1e-9) / before.max(1e-9)).log10();
    assert!(
        total_db < -12.0,
        "EQ가 실제로 걸리지 않았다(총 변화 {total_db:.1}dB) — 테스트가 무의미하다"
    );

    // 계수를 즉시 갈아끼우면 18dB가 사실상 첫 창에 다 들어간다.
    // 부드럽게 옮기면 한 창당 3dB를 넘지 않는다.
    assert!(
        worst_step_db < 3.0,
        "EQ 변경이 2.7ms 안에 {worst_step_db:.1}dB 꺾인다(딸깍 소리)"
    );
}

#[test]
fn eq_smoothing_actually_reaches_the_target() {
    let mut dsp = ChannelDspState::new();

    let target = vec![band(120.0, -12.0)];
    dsp.update_eq_targets(&target, FS);

    // 목표만 설정됐을 뿐 아직 반영 전이어야 한다(스냅하지 않는다는 증거).
    assert!(
        (dsp.current_bands[0].freq - 120.0).abs() > 1.0,
        "update_eq_targets가 계수를 즉시 스냅했다"
    );

    // 충분한 시간 뒤에는 목표에 도달해야 한다. 도달하지 못하면 EQ가
    // 영원히 어긋난 채로 남는다. 0.5초(약 375스텝)면 충분하다.
    for _ in 0..375 {
        dsp.smooth_eq_step(FS);
    }
    assert!(
        (dsp.current_bands[0].freq - 120.0).abs() < 0.01,
        "주파수가 목표에 도달하지 못했다: {}",
        dsp.current_bands[0].freq
    );
    assert!(
        (dsp.current_bands[0].gain - (-12.0)).abs() < 0.01,
        "게인이 목표에 도달하지 못했다: {}",
        dsp.current_bands[0].gain
    );
    assert!(dsp.current_bands[0].enabled);
}
