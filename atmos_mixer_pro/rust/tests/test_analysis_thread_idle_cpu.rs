//! 분석 스레드가 유휴 상태에서 코어를 태우지 않는지 확인하는 회귀 테스트.
//!
//! rtrb의 `read_chunk(0)`은 "0슬롯 읽기"로 **성공**한다. 예전 코드는
//! `rx.read_chunk(rx.slots())`를 그대로 호출해서 링버퍼가 비어 있어도 Ok
//! 분기로 들어갔고, 빈 버퍼로 ebur128 loudness 질의를 돌린 뒤 sleep 없이
//! 곧바로 다시 돌았다. 5ms sleep이 있는 else 분기는 사실상 죽은 코드였다.
//!
//! 결과적으로 무음 상태에서도 이 스레드가 코어 하나를 100% 점유했다. 실시간
//! 오디오 콜백과 CPU를 다투게 되므로 드롭아웃(잡음)의 원인이 되고, 24시간
//! 무중단 전시 설치에서는 발열/전력 문제이기도 하다.
//!
//! 이 테스트는 스레드를 띄워놓고 아무 데이터도 넣지 않은 뒤, 이 프로세스가
//! 실제로 소비한 CPU 시간을 측정한다. 바쁜 회전이면 측정 구간과 비슷한 CPU
//! 시간을 쓰고, 제대로 쉬면 거의 쓰지 않는다.

use rust_lib_atmos_mixer_pro::audio::analysis::start_analysis_thread;
use std::time::{Duration, Instant};

/// 이 프로세스가 지금까지 쓴 사용자+시스템 CPU 시간.
fn process_cpu_time() -> Duration {
    // getrusage(RUSAGE_SELF)
    unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        assert_eq!(libc::getrusage(libc::RUSAGE_SELF, &mut usage), 0);
        let u = Duration::new(
            usage.ru_utime.tv_sec as u64,
            (usage.ru_utime.tv_usec as u32) * 1000,
        );
        let s = Duration::new(
            usage.ru_stime.tv_sec as u64,
            (usage.ru_stime.tv_usec as u32) * 1000,
        );
        u + s
    }
}

#[test]
fn analysis_thread_does_not_spin_when_ring_buffer_is_empty() {
    let (_producer, consumer) = rtrb::RingBuffer::<f32>::new(65536);

    // 프로듀서를 붙잡고 아무것도 쓰지 않는다 = 항상 빈 링버퍼(유휴 상태).
    start_analysis_thread(consumer, 48000, 16);

    // 스레드가 자리잡을 시간을 준다.
    std::thread::sleep(Duration::from_millis(200));

    let cpu_before = process_cpu_time();
    let wall_start = Instant::now();

    std::thread::sleep(Duration::from_millis(1000));

    let cpu_used = process_cpu_time() - cpu_before;
    let wall = wall_start.elapsed();

    let ratio = cpu_used.as_secs_f64() / wall.as_secs_f64();

    // 바쁜 회전이면 코어 하나를 통째로 쓰므로 비율이 1.0에 가깝다.
    // 제대로 쉬면 5ms 주기로 깨어나기만 하므로 수 % 수준이다.
    assert!(
        ratio < 0.25,
        "유휴 상태에서 분석 스레드가 CPU를 태우고 있다. \
         CPU {:?} / 벽시계 {:?} = {:.1}% (바쁜 회전이면 100%에 근접)",
        cpu_used,
        wall,
        ratio * 100.0
    );
}
