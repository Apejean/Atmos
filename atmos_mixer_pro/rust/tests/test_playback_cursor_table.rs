//! 재생 위치 표의 읽기·쓰기 규약: 오디오 스레드는 기다리지 않고, 읽는 쪽은 칸 주인이 바뀌는
//! 순간에 걸려도 (id, 위치) 짝을 섞지 않는다.
use rust_lib_atmos_mixer_pro::audio::playback_cursor::CursorTable;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[test]
fn 쓰고_지우고_다시_쓰면_읽는_쪽에_그대로_보인다() {
    let t = CursorTable::new(8);
    t.publish(3, 77, 1.5);
    assert_eq!(t.snapshot(), vec![(77, 1.5)]);
    t.publish(3, 77, 2.0);
    assert_eq!(t.snapshot(), vec![(77, 2.0)]);
    t.clear(3);
    assert!(t.snapshot().is_empty());
    t.publish(3, 78, 0.25);
    t.publish(9, 99, 1.0); // 범위 밖 칸은 무시한다
    assert_eq!(t.snapshot(), vec![(78, 0.25)]);
    t.clear_all();
    assert!(t.snapshot().is_empty());
}

#[test]
fn 칸_주인이_바뀌는_중에도_id와_위치_짝이_섞이지_않는다() {
    // 실제 엔진처럼 칸 주인 id는 매번 새 값이다(인스턴스 id는 고유하다). 위치 = id 값으로 적어
    // 읽은 짝이 같은 주인의 것인지 확인한다.
    let t = Arc::new(CursorTable::new(1));
    let stop = Arc::new(AtomicBool::new(false));
    let writer = {
        let (t, stop) = (t.clone(), stop.clone());
        std::thread::spawn(move || {
            let mut id = 1u64;
            while !stop.load(Ordering::Relaxed) {
                t.publish(0, id, id as f64);
                t.publish(0, id, id as f64); // 같은 주인이 위치만 갱신하는 블록
                id += 1;
            }
        })
    };
    let deadline = Instant::now() + Duration::from_millis(300);
    let mut seen = 0usize;
    while Instant::now() < deadline {
        for (id, s) in t.snapshot() {
            seen += 1;
            assert_eq!(s, id as f64, "짝이 섞였다: id {id}, 위치 {s}");
        }
    }
    stop.store(true, Ordering::Relaxed);
    writer.join().unwrap();
    assert!(seen > 0, "한 번도 읽지 못했다");
}
