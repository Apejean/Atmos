//! 채널 리버브 센드가 실제 소리에 반영되는지 고정한다.
//!
//! 예전에는 SetChannelReverbSend가 target_reverb_send에 값을 쓰기만 하고
//! 오디오 경로에서 한 번도 읽지 않았다. 그래서 스피커 인스펙터에서 센드를
//! 바꿔도 채널 리버브는 랙 설정 그대로 걸렸다.
//!
//! 규약: 출력 = 원음 + (리버브 통과음 - 원음) x 센드
//!   센드 0 -> 원음 그대로(잔향 없음), 센드 1 -> 랙 설정 그대로.
//! 센드는 잔향(late reverb)의 양만 조절하고 초기 반사에는 관여하지 않는다.

use rust_lib_atmos_mixer_pro::audio::dsp::dsp_utils::ChannelDspState;

const FS: f32 = 48_000.0;
const INPUT_SEC: f32 = 0.2;
const TAIL_SEC: f32 = 1.0;

fn noise(seed: &mut u32) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    (*seed as f32 / u32::MAX as f32) * 2.0 - 1.0
}

/// 랙 설정: 800m3, 2초, 프리딜레이 10ms, 댐핑 40%, 밀도 80%, MIX 100%.
/// `reverb_on`이 false면 같은 파라미터로 리버브만 끈다.
fn channel(reverb_on: bool) -> ChannelDspState {
    let mut dsp = ChannelDspState::new();
    dsp.reverb
        .set_full_params(reverb_on, 800.0, 2.0, 10.0, 40.0, 80.0, 1.0);
    dsp
}

/// 0.2초 잡음을 넣고 이어서 1초 무음을 넣은 출력 전체.
/// `send`가 None이면 센드를 건드리지 않은 기본 상태로 잰다.
fn render(reverb_on: bool, send: Option<f32>) -> Vec<f32> {
    let mut dsp = channel(reverb_on);
    if let Some(s) = send {
        // 스무딩 램프를 건너뛰고 정상 상태에서 잰다.
        dsp.target_reverb_send = s;
        dsp.current_reverb_send = s;
    }
    let mut seed = 0x1234_5678u32;
    let n_in = (FS * INPUT_SEC) as usize;
    let n_tail = (FS * TAIL_SEC) as usize;
    let mut out = Vec::with_capacity(n_in + n_tail);
    for _ in 0..n_in {
        out.push(dsp.process(noise(&mut seed) * 0.5, FS));
    }
    for _ in 0..n_tail {
        out.push(dsp.process(0.0, FS));
    }
    out
}

/// 입력이 끝난 뒤 1초 동안의 에너지.
fn tail_energy(send: Option<f32>) -> f64 {
    let y = render(true, send);
    let start = (FS * INPUT_SEC) as usize;
    y[start..].iter().map(|v| (*v as f64) * (*v as f64)).sum()
}

#[test]
fn 센드_0이면_리버브를_끈_것과_같은_소리다() {
    // 센드 0은 "잔향 없음"이면서 원음은 그대로여야 한다(레벨이 줄면 안 된다).
    //
    // 처음에는 "센드 0이면 입력이 끝난 뒤 출력이 거의 0"으로 검사했는데,
    // 입력이 끊긴 뒤에도 채널 원음 경로의 IIR 필터(DC 블로커, EQ)가 짧은
    // 여운을 남긴다(실측: 센드0 0.0044 vs 센드1 368.7, 약 -49dB). 그 여운은
    // 리버브가 아니므로, 리버브를 끈 채널과 샘플 단위로 같은지 보는 것이 맞는
    // 기준이다. 잔향 유무와 원음 레벨 보존을 함께 검사한다.
    let full = tail_energy(Some(1.0));
    assert!(full > 1e-3, "기준 잔향이 너무 작다(테스트 설정 오류): {full}");

    let send0 = render(true, Some(0.0));
    let off = render(false, None);
    assert_eq!(send0.len(), off.len());
    let max_diff = send0
        .iter()
        .zip(off.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    assert!(
        max_diff < 1e-9,
        "센드 0인데 리버브를 끈 소리와 다르다: 최대 차이 {max_diff:e}"
    );
}

#[test]
fn 센드_절반이면_잔향_진폭도_절반이다() {
    let full = tail_energy(Some(1.0));
    let half = tail_energy(Some(0.5));
    let ratio = half / full; // 진폭 0.5 -> 에너지 0.25
    assert!(
        (ratio - 0.25).abs() < 0.02,
        "센드 0.5의 잔향 에너지 비율이 {ratio:.4} (기대 0.25)"
    );
}

#[test]
fn 센드를_받기_전_기본값은_랙_설정을_그대로_따른다() {
    // 인스펙터가 센드를 보내기 전(엔진 기동 직후, 스피커가 없는 채널)에도
    // 예전처럼 랙 설정대로 리버브가 걸려야 한다.
    let default = tail_energy(None);
    let full = tail_energy(Some(1.0));
    assert!(
        (default / full - 1.0).abs() < 1e-3,
        "기본 센드가 1.0이 아니다: 기본={default:.6} 센드1={full:.6}"
    );
}

#[test]
fn 센드_변경은_즉시_점프하지_않는다() {
    // DSP Law 3: 파라미터를 순간 전환하면 딸깍 소리가 난다.
    let mut dsp = channel(true);
    dsp.target_reverb_send = 1.0;
    dsp.current_reverb_send = 1.0;
    dsp.target_reverb_send = 0.0;
    dsp.process(0.1, FS);
    assert!(
        dsp.current_reverb_send > 0.9,
        "한 샘플 만에 센드가 {}로 뛰었다",
        dsp.current_reverb_send
    );
    for _ in 0..(FS * 0.1) as usize {
        dsp.process(0.0, FS);
    }
    assert!(
        dsp.current_reverb_send < 1e-3,
        "100ms 뒤에도 목표에 도달하지 못했다: {}",
        dsp.current_reverb_send
    );
}
