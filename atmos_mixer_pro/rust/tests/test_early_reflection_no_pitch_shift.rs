//! 스피커를 드래그하면 반사 경로 길이가 바뀌어 초기반사음 탭의 목표 딜레이가
//! 계속 갱신된다. 예전에는 탭 딜레이를 1-pole로 **미끄러뜨려** 읽기 위치가
//! 연속으로 움직였고, 그건 테이프 속도를 바꾸는 것과 같아 반사음 피치가
//! 휘었다(실기 보고: "스피커를 움직이면 빨리감기 같은 소리").
//!
//! 반사음 성분만 떼어내(초기반사 mix=1 출력 - mix=0 출력) 1kHz 주기가
//! 유지되는지 본다.

use rust_lib_atmos_mixer_pro::audio::dsp::dsp_utils::ChannelDspState;

const FS: f32 = 48_000.0;
const PERIOD: f32 = 48.0; // 1kHz @ 48kHz

fn prepare(mix: f32) -> ChannelDspState {
    let mut s = ChannelDspState::new();
    for tap in s.taps.iter_mut() {
        tap.target_gain = 0.0;
        tap.current_gain = 0.0;
        tap.target_delay_ms = 10.0;
        tap.current_delay_ms = 10.0;
    }
    s.taps[0].target_gain = 1.0;
    s.taps[0].current_gain = 1.0;
    s.target_early_ref_mix = mix;
    s.current_early_ref_mix = mix;
    s
}

#[test]
fn moving_reflection_tap_keeps_pitch() {
    let mut with_er = prepare(1.0);
    let mut without_er = prepare(0.0);

    let mut phase = 0.0f32;
    let mut er: Vec<f32> = Vec::new();
    let total = 4_800 + 9_600;
    for n in 0..total {
        // 워밍업 뒤 약 16ms마다 탭 딜레이를 0.75ms씩 옮긴다(드래그).
        // 0.75ms = 3/4주기라 크로스페이드 중 두 탭이 상쇄되지 않는다.
        if n >= 4_800 && (n - 4_800) % 800 == 0 {
            let next = with_er.taps[0].target_delay_ms + 0.75;
            with_er.taps[0].target_delay_ms = next;
            without_er.taps[0].target_delay_ms = next;
        }
        let x = phase.sin() * 0.5;
        phase += 2.0 * std::f32::consts::PI * 1000.0 / FS;
        let a = with_er.process(x, FS);
        let b = without_er.process(x, FS);
        if n >= 4_800 {
            er.push(a - b);
        }
    }

    // 양의 방향 영교차 사이 간격 = 주기.
    //
    // 크로스페이드 도중 두 탭의 위상이 반 주기 어긋나면 순간적으로 상쇄되어
    // 진폭이 0 근처로 떨어진다. 그 구간의 영교차는 잡음이라 주기를 말해주지
    // 않으므로(피치가 아니라 진폭 변화), 신호가 실제로 있는 주기만 센다.
    // 미끄러지는 옛 코드는 진폭이 온전한 채로 주기가 틀어지므로 여전히 잡힌다.
    let mut last: Option<usize> = None;
    let mut worst = 0.0f32;
    let mut counted = 0usize;
    let mut peak = 0.0f32;
    for i in 1..er.len() {
        peak = peak.max(er[i].abs());
        if er[i - 1] <= 0.0 && er[i] > 0.0 {
            if let Some(l) = last {
                if peak > 0.15 {
                    counted += 1;
                    let dev = ((i - l) as f32 - PERIOD).abs();
                    if dev > worst {
                        worst = dev;
                    }
                }
            }
            last = Some(i);
            peak = 0.0;
        }
    }
    assert!(counted >= 100, "측정한 주기가 {counted}개뿐이다 — 테스트가 무의미하다");

    assert!(
        worst <= 3.0,
        "탭 딜레이를 옮기는 동안 반사음 주기가 {worst:.1}샘플 틀어졌다 \
         (1kHz 기준 48샘플) — 읽기 위치가 미끄러지며 피치가 휜다"
    );
}
