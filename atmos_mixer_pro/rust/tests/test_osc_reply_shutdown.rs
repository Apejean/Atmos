//! OSC 응답 송신을 닫은 뒤에는 송신 스레드가 다시 생기지 않고 아무것도 보내지 않는다(osc::reply::shutdown).
//! 전역 상태를 닫으므로 다른 시험과 파일을 나눈다.
use rust_lib_atmos_mixer_pro::common::config::AppConfig;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use rust_lib_atmos_mixer_pro::osc::reply::*;
use std::net::UdpSocket;
use std::time::Duration;

#[test]
fn shutdown_stops_sending_and_does_not_respawn() {
    let rx = UdpSocket::bind("127.0.0.1:0").unwrap();
    rx.set_read_timeout(Some(Duration::from_millis(600))).unwrap();
    let port = rx.local_addr().unwrap().port();
    *GLOBAL_STATE.config.write().unwrap() = Some(AppConfig {
        osc_reply_targets: vec![format!("127.0.0.1:{port}")],
        ..AppConfig::default()
    });
    let mut buf = [0u8; 512];

    send_reply("/before", ReplyStage::Ack, "");
    assert!(rx.recv_from(&mut buf).is_ok(), "닫기 전에는 보낸다");

    shutdown();
    assert!(is_closed());
    send_reply("/after", ReplyStage::Ack, "");
    assert!(rx.recv_from(&mut buf).is_err(), "닫은 뒤에는 보내지 않는다");
}

#[test]
fn play_error_reason_codes() {
    assert_eq!(reason_for_play_error("파일을 찾을 수 없습니다: C:/x.wav"), reason::FILE_MISSING);
    assert_eq!(reason_for_play_error("Failed to open stream"), reason::PLAY_FAILED);
}
