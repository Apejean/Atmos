//! OSC 수신 포트 열기 재시도(검토 보고서 A7). 전역 리스너 상태를 쓰므로 이 파일의 시험은 하나다.
//! - 다른 소켓이 포트를 잡고 있으면 막힘 상태가 되고, 놓으면 사람 없이 다시 붙는다.
//! - 리스너를 다른 포트로 다시 시작하면, 옛 포트를 기다리던 스레드는 끝나 옛 포트를 잡지 않는다.
use rust_lib_atmos_mixer_pro::api::simple::api_start_osc_listener;
use rust_lib_atmos_mixer_pro::osc::listener::{bind_retry_delay, current_port_status};
use std::net::UdpSocket;
use std::time::{Duration, Instant};

fn free_port_held() -> (UdpSocket, u16) {
    let s = UdpSocket::bind("0.0.0.0:0").unwrap();
    let p = s.local_addr().unwrap().port();
    (s, p)
}

fn wait_for(what: &str, limit: Duration, mut cond: impl FnMut() -> bool) {
    let end = Instant::now() + limit;
    while !cond() {
        assert!(Instant::now() < end, "{what}: {limit:?} 안에 되지 않았다");
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn retry_delays_are_1_2_5_then_10_seconds() {
    let secs: Vec<u64> = (1..=6).map(|n| bind_retry_delay(n).as_secs()).collect();
    assert_eq!(secs, vec![1, 2, 5, 10, 10, 10]);
}

#[test]
fn blocked_port_recovers_and_restart_ends_old_waiters() {
    // 1) 막혔다가 풀리면 붙는다.
    let (blocker, port) = free_port_held();
    api_start_osc_listener(port);
    let blocked = format!("OscPortBlocked:{port}");
    wait_for("막힘 상태", Duration::from_secs(3), || current_port_status().as_deref() == Some(blocked.as_str()));
    drop(blocker);
    wait_for("다시 붙음", Duration::from_secs(5), || current_port_status().is_none());
    assert!(
        UdpSocket::bind(("0.0.0.0", port)).is_err(),
        "리스너가 포트를 다시 잡고 있어야 한다"
    );

    // 2) 막힌 포트 A를 기다리는 중에 다른 포트 B로 다시 시작하면, A를 기다리던 스레드는 끝난다.
    let (blocker_a, port_a) = free_port_held();
    api_start_osc_listener(port_a);
    let blocked_a = format!("OscPortBlocked:{port_a}");
    wait_for("A 막힘", Duration::from_secs(3), || current_port_status().as_deref() == Some(blocked_a.as_str()));
    let (probe_b, port_b) = free_port_held();
    drop(probe_b); // B는 비어 있다
    api_start_osc_listener(port_b);
    assert_eq!(current_port_status(), None, "새 포트로 시작하면 옛 막힘 표시는 내린다");
    drop(blocker_a);
    // A의 옛 스레드가 살아 있었다면 1·2초 뒤 재시도에서 A를 잡는다. 충분히 기다린 뒤 A가 비어 있어야 한다.
    std::thread::sleep(Duration::from_millis(3500));
    assert!(
        UdpSocket::bind(("0.0.0.0", port_a)).is_ok(),
        "다시 시작한 뒤에는 옛 포트를 잡지 않는다"
    );
    assert_eq!(current_port_status(), None);
}
