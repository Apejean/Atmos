// tests/test_crossover_bass_management.rs
// Atmos Mixer Pro - 베이스 매니지먼트 크로스오버 대역 반전 회귀 테스트
//
// 배경: audio::crossover::LinkwitzRiley24 (프로덕션 서브우퍼 크로스오버)에서
// hpf*/lpf* 필드에 EqType이 뒤바뀌어 들어가 process_low()가 고역을,
// process_high()가 저역을 출력하는 결함이 있었다 (서브우퍼가 고역을 받고
// 새틀라이트가 저역 전체를 받아 스피커 파손 위험). 이 테스트는 반드시
// 프로덕션이 실제로 사용하는 crossover::LinkwitzRiley24를 대상으로 해야 한다
// (svf::LinkwitzRiley24는 이름만 같은 별개의 테스트용 구현이라 이 결함을 잡지 못했다).

use rust_lib_atmos_mixer_pro::audio::crossover::LinkwitzRiley24;

const SAMPLE_RATE: f32 = 48000.0;
const CROSSOVER_FREQ: f32 = 80.0;

/// 지정한 주파수의 사인파를 크로스오버에 통과시켜 정상상태 RMS를 반환한다.
/// 초반 필터 정착 구간(0.5초)은 RMS 계산에서 제외한다.
fn steady_state_rms(freq: f32, use_low: bool) -> f32 {
    let mut lr24 = LinkwitzRiley24::new();
    lr24.set_crossover_freq(CROSSOVER_FREQ, SAMPLE_RATE);

    let settle_samples = (SAMPLE_RATE * 0.5) as usize;
    let measure_samples = (SAMPLE_RATE * 0.5) as usize;

    let mut sum_sq = 0.0f32;
    for i in 0..(settle_samples + measure_samples) {
        let t = i as f32 / SAMPLE_RATE;
        let input = (2.0 * std::f32::consts::PI * freq * t).sin();
        let out = if use_low {
            lr24.process_low(input)
        } else {
            lr24.process_high(input)
        };

        if i >= settle_samples {
            sum_sq += out * out;
        }
    }

    (sum_sq / measure_samples as f32).sqrt()
}

#[test]
fn test_crossover_process_low_passes_bass_not_treble() {
    // process_low()는 저역(40Hz)을 그대로 통과시키고, 고역(2kHz)은 감쇠시켜야 한다.
    let rms_40hz = steady_state_rms(40.0, true);
    let rms_2khz = steady_state_rms(2000.0, true);

    println!("process_low(): 40Hz RMS={:.4}, 2kHz RMS={:.4}", rms_40hz, rms_2khz);

    assert!(
        rms_40hz > 0.5,
        "process_low()가 40Hz 저역을 통과시키지 못함! RMS={}",
        rms_40hz
    );
    assert!(
        rms_2khz < 0.1,
        "process_low()가 2kHz 고역을 감쇠시키지 못함 (크로스오버 반전 의심)! RMS={}",
        rms_2khz
    );
}

#[test]
fn test_crossover_process_high_passes_treble_not_bass() {
    // process_high()는 고역(2kHz)을 그대로 통과시키고, 저역(40Hz)은 감쇠시켜야 한다.
    let rms_40hz = steady_state_rms(40.0, false);
    let rms_2khz = steady_state_rms(2000.0, false);

    println!("process_high(): 40Hz RMS={:.4}, 2kHz RMS={:.4}", rms_40hz, rms_2khz);

    assert!(
        rms_2khz > 0.5,
        "process_high()가 2kHz 고역을 통과시키지 못함! RMS={}",
        rms_2khz
    );
    assert!(
        rms_40hz < 0.1,
        "process_high()가 40Hz 저역을 감쇠시키지 못함 (크로스오버 반전 의심)! RMS={}",
        rms_40hz
    );
}
