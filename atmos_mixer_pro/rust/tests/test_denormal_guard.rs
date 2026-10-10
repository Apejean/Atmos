//! 오디오 콜백의 비정규 실수(denormal) 가드(audio::denormal).
//!
//! 2026-10-09 Windows 측정: 소리가 멈춘 뒤 리버브·필터 꼬리가 비정규 실수가 되자 콜백 처리 시간이
//! 평균 11ms에서 최대 98ms까지 늘어 예산 21.3ms(1024프레임)를 넘었고 소리가 늘어졌다. 가드를 켜면
//! 내내 11ms였다. 여기서는 가드의 동작과, 엔진의 모든 출력 콜백이 가드를 만드는지 고정한다.

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use rust_lib_atmos_mixer_pro::audio::denormal::DenormalGuard;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use std::hint::black_box;

/// 가장 작은 정규 f32를 4로 나눈 값. FTZ·DAZ가 꺼져 있으면 비정규 실수, 켜져 있으면 0이다.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn quarter_of_min_normal() -> f32 {
    black_box(f32::MIN_POSITIVE) / black_box(4.0f32)
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[test]
fn 가드_안에서는_비정규_실수가_0이_되고_끝나면_원래대로_돌아온다() {
    let before = quarter_of_min_normal();
    assert!(
        before > 0.0 && before < f32::MIN_POSITIVE,
        "테스트 스레드는 원래 비정규 실수를 그대로 다뤄야 한다: {before:e}"
    );
    {
        let _guard = DenormalGuard::new();
        assert_eq!(quarter_of_min_normal(), 0.0, "가드 안에서 비정규 실수가 남았다");
    }
    let after = quarter_of_min_normal();
    assert!(
        after > 0.0,
        "가드가 끝난 뒤에도 FTZ가 남았다(드라이버 스레드의 부동소수점 상태를 바꾼다): {after:e}"
    );
}

#[test]
fn 엔진의_모든_출력_콜백이_가드를_만든다() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/audio/engine.rs");
    let src = std::fs::read_to_string(path).expect("engine.rs를 읽지 못했다");
    let callbacks = src.matches("device.build_output_stream(").count();
    let guards = src.matches("DenormalGuard::new()").count();
    assert!(callbacks >= 4, "출력 콜백을 찾지 못했다(구조가 바뀌었다): {callbacks}");
    assert_eq!(
        guards, callbacks,
        "출력 콜백 {callbacks}개 중 가드를 만드는 것은 {guards}개다"
    );
}
