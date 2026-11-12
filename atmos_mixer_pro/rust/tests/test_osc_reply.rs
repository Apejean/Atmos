// OSC 응답 송신(osc::reply, 규격 OSC_INTERFACE_SPEC.md §3): 목적지 검사, 메시지 형식, 실제 UDP 송신, 옛 설정 호환.

use rosc::{OscPacket, OscType};
use rust_lib_atmos_mixer_pro::common::config::AppConfig;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use rust_lib_atmos_mixer_pro::osc::reply::*;
use std::net::UdpSocket;
use std::time::Duration;

#[test]
fn check_target_accepts_numeric_ip_port() {
    assert!(check_target("192.168.0.10:9000").is_ok());
    assert!(check_target(" 127.0.0.1:53001 ").is_ok());
    assert!(check_target("[::1]:9000").is_ok());
}

#[test]
fn check_target_rejects_bad_entries() {
    assert_eq!(check_target("controller.local:9000"), Err("not_ip_port"));
    assert_eq!(check_target("192.168.0.10"), Err("not_ip_port"));
    assert_eq!(check_target("192.168.0.10:0"), Err("port_zero"));
    assert_eq!(check_target("0.0.0.0:9000"), Err("unspecified"));
    assert_eq!(check_target("[::]:9000"), Err("unspecified"));
    assert_eq!(check_target("239.1.2.3:9000"), Err("multicast"));
    assert_eq!(check_target("255.255.255.255:9000"), Err("broadcast"));
}

#[test]
fn parse_targets_limits_dedupes_and_reports() {
    let mut list: Vec<String> = (1..=10).map(|i| format!("10.0.0.{i}:9000")).collect();
    list.insert(0, "10.0.0.1:9000".to_string()); // 중복
    list.push("bad".to_string());
    list.push("   ".to_string()); // 빈칸은 무시
    let (ok, bad) = parse_targets(&list);
    assert_eq!(ok.len(), MAX_TARGETS);
    assert!(bad.iter().any(|(t, why)| t == "bad" && *why == "not_ip_port"));
    assert_eq!(bad.iter().filter(|(_, why)| *why == "over_limit").count(), 2);
}

#[test]
fn reply_message_format() {
    let m = reply_message("/room1/track2/play", ReplyStage::Rejected, reason::GATED);
    assert_eq!(m.addr, "/awesome/reply");
    assert_eq!(
        m.args,
        vec![
            OscType::String("/room1/track2/play".into()),
            OscType::String("rejected".into()),
            OscType::String("gated".into()),
        ]
    );
    assert_eq!(ReplyStage::Ack.as_str(), "ack");
    assert_eq!(ReplyStage::Started.as_str(), "started");
}

#[test]
fn boot_message_has_six_args_in_spec_order() {
    let m = boot_message(&BootInfo {
        version: "1.3.11",
        engine_state: "Running",
        device: "ASIO MADIface USB",
        start_kind: "relaunch",
        resumed_tracks: 4,
        boot_id: "1762900000000",
    });
    assert_eq!(m.addr, "/awesome/status/boot");
    assert_eq!(
        m.args,
        vec![
            OscType::String("1.3.11".into()),
            OscType::String("Running".into()),
            OscType::String("ASIO MADIface USB".into()),
            OscType::String("relaunch".into()),
            OscType::Int(4),
            OscType::String("1762900000000".into()),
        ]
    );
}

#[test]
fn unknown_start_kind_is_manual() {
    assert_eq!(normalize_start_kind("logon"), ("logon", true));
    assert_eq!(normalize_start_kind("relaunch"), ("relaunch", true));
    assert_eq!(normalize_start_kind("manual"), ("manual", true));
    assert_eq!(normalize_start_kind("boot"), ("manual", false));
}

#[test]
fn snapshot_truncates_long_track_list_but_keeps_count() {
    let ids: Vec<String> = (0..200).map(|i| format!("room_{i:03}/track_{i:03}")).collect();
    let m = snapshot_message("Running", "", &ids, 40);
    assert_eq!(m.addr, "/awesome/status/snapshot");
    assert_eq!(m.args[2], OscType::Int(200));
    let OscType::String(list) = &m.args[3] else { panic!("list arg") };
    assert!(list.len() <= SNAPSHOT_LIST_MAX_BYTES + ",…".len());
    assert!(list.ends_with(",…"));
    assert_eq!(m.args[4], OscType::Int(40));
    let short = join_track_list(&["a/1".to_string(), "b/2".to_string()]);
    assert_eq!(short, "a/1,b/2");
}

#[test]
fn old_config_without_field_reads_empty_targets() {
    let old = r#"{"osc_port":8000,"device_name":null,"buffer_size":256}"#;
    let c: AppConfig = serde_json::from_str(old).expect("old config");
    assert!(c.osc_reply_targets.is_empty());
    // 저장할 때는 사용자가 적은 문자열을 그대로 둔다(잘못된 항목도).
    let mut c2 = c.clone();
    c2.osc_reply_targets = vec!["bad".into(), "10.0.0.1:9000".into()];
    let back: AppConfig = serde_json::from_str(&serde_json::to_string(&c2).unwrap()).unwrap();
    assert_eq!(back.osc_reply_targets, vec!["bad".to_string(), "10.0.0.1:9000".to_string()]);
}

#[test]
fn reply_reaches_configured_udp_target() {
    let rx = UdpSocket::bind("127.0.0.1:0").unwrap();
    rx.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    let port = rx.local_addr().unwrap().port();
    *GLOBAL_STATE.config.write().unwrap() = Some(AppConfig {
        osc_reply_targets: vec!["bad-entry".into(), format!("127.0.0.1:{port}")],
        ..AppConfig::default()
    });

    send_reply("/room1/track1/play", ReplyStage::Ack, "");

    let mut buf = [0u8; 2048];
    let (n, _) = rx.recv_from(&mut buf).expect("reply packet");
    let (_, packet) = rosc::decoder::decode_udp(&buf[..n]).unwrap();
    let OscPacket::Message(m) = packet else { panic!("message") };
    assert_eq!(m.addr, "/awesome/reply");
    assert_eq!(m.args[1], OscType::String("ack".into()));
}
