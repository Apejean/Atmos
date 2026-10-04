//! 엔진이 사라지면(워치독 재시작·엔진 정지) 그 엔진의 분석 스레드도 끝나야 한다.
//!
//! 엔진은 시작할 때마다 분석 스레드를 새로 띄운다(engine.rs start). 예전 스레드는 링버퍼의
//! 생산자(엔진의 믹서)가 사라져도 루프를 빠져나오지 않아서, 5ms마다 깨어나는 스레드가 재시작마다
//! 하나씩 쌓였다(장시간 무인 운영에서 스레드·CPU 누적).

use rust_lib_atmos_mixer_pro::audio::analysis::start_analysis_thread;
use std::time::{Duration, Instant};

#[test]
fn 생산자가_사라지면_분석_스레드가_끝난다() {
    let (producer, consumer) = rtrb::RingBuffer::<f32>::new(65536);
    let handle = start_analysis_thread(consumer, 48_000, 16);

    std::thread::sleep(Duration::from_millis(100));
    assert!(!handle.is_finished(), "생산자가 살아 있는데 분석 스레드가 끝났다");

    drop(producer);
    let t0 = Instant::now();
    while !handle.is_finished() && t0.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(handle.is_finished(), "생산자(엔진)가 사라졌는데 분석 스레드가 계속 돈다");
}
