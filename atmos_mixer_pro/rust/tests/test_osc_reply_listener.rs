//! OSC 응답 연결 시험(규격 OSC_INTERFACE_SPEC.md §3.4·§3.6, 방 클리어 응답 포함). 실제 OSC 리스너에 UDP로 보내고,
//! 설정의 응답 목적지(이 시험의 소켓)로 오는 `/awesome/reply`·`/awesome/status/snapshot`을 읽는다.
//! 전역 상태를 쓰므로 이 파일의 시험은 하나다.
use rosc::{encoder, OscMessage, OscPacket, OscType};
use rust_lib_atmos_mixer_pro::api::simple::{api_set_active_room, api_start_osc_listener, ENGINE_ACTIVE};
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::net::UdpSocket;
use std::sync::atomic::Ordering;
use std::time::Duration;

fn tone_wav(path: &std::path::Path, seconds: f32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 48_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for i in 0..(48_000.0 * seconds) as usize {
        let v = (i as f32 * 200.0 * std::f32::consts::TAU / 48_000.0).sin() * 3000.0;
        w.write_sample(v as i16).unwrap();
    }
    w.finalize().unwrap();
}

fn track(room: &str, id: &str, path: &str) -> TrackConfig {
    TrackConfig {
        id: id.into(),
        name: id.into(),
        file_path: path.into(),
        volume: 1.0,
        is_loop: true,
        is_streaming: false,
        output_channel: 1,
        output_stereo: false,
        play_osc_address: format!("/{room}/{id}/play"),
        stop_osc_address: format!("/{room}/{id}/stop"),
    }
}

fn room(id: &str, tracks: Vec<TrackConfig>) -> RoomConfig {
    RoomConfig {
        id: id.into(),
        name: id.into(),
        color_hex: "#ffffff".into(),
        volume: 1.0,
        clear_osc_address: format!("/{id}/clear"),
        tracks,
    }
}

/// 응답 소켓에서 메시지 하나를 읽는다.
fn recv(rx: &UdpSocket) -> OscMessage {
    let mut buf = [0u8; 4096];
    let (n, _) = rx.recv_from(&mut buf).unwrap_or_else(|e| panic!("응답이 와야 한다: {e:?}"));
    match rosc::decoder::decode_udp(&buf[..n]).unwrap().1 {
        OscPacket::Message(m) => m,
        OscPacket::Bundle(_) => panic!("번들이 아니라 메시지여야 한다"),
    }
}

fn reply(rx: &UdpSocket) -> (String, String, String) {
    let m = recv(rx);
    assert_eq!(m.addr, "/awesome/reply");
    let s = |i: usize| match &m.args[i] {
        OscType::String(v) => v.clone(),
        other => panic!("문자열 인자여야 한다: {other:?}"),
    };
    (s(0), s(1), s(2))
}

#[test]
fn 실제_리스너는_명령마다_응답하고_상태_조회에_답한다() {
    let dir = std::env::temp_dir().join(format!("atmos_osc_reply_listener_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    tone_wav(&dir.join("ok.wav"), 3.0);
    let ok_path = dir.join("ok.wav").to_string_lossy().into_owned();
    let gone_path = dir.join("gone.wav").to_string_lossy().into_owned(); // 만들지 않는다
    GLOBAL_STATE.engine_sample_rate.store(48_000, Ordering::SeqCst);

    let rx = UdpSocket::bind("127.0.0.1:0").unwrap();
    rx.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    let reply_port = rx.local_addr().unwrap().port();
    let port = UdpSocket::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();

    *GLOBAL_STATE.config.write().unwrap() = Some(AppConfig {
        osc_port: port,
        is_exhibition_mode: false,
        osc_reply_targets: vec![format!("127.0.0.1:{reply_port}")],
        rooms: vec![
            room("a", vec![track("a", "ok", &ok_path), track("a", "gone", &gone_path)]),
            room("b", vec![track("b", "x", &ok_path)]),
        ],
        ..AppConfig::default()
    });
    GLOBAL_STATE.config_version.fetch_add(1, Ordering::SeqCst);
    GLOBAL_STATE.clear_playing_tracks();
    api_start_osc_listener(port);
    std::thread::sleep(Duration::from_millis(300));
    api_set_active_room(Some("a".into())).unwrap();
    // 장치 없이 시험한다. 엔진이 도는 것으로 표시해야 started가 온다.
    ENGINE_ACTIVE.store(true, Ordering::SeqCst);

    let sock = UdpSocket::bind("127.0.0.1:0").unwrap();
    let send = |addr: &str| {
        let msg = OscPacket::Message(OscMessage { addr: addr.into(), args: vec![] });
        sock.send_to(&encoder::encode(&msg).unwrap(), ("127.0.0.1", port)).unwrap();
    };

    // 1) 재생: ack → started
    send("/a/ok/play");
    assert_eq!(reply(&rx), ("/a/ok/play".into(), "ack".into(), "".into()));
    assert_eq!(reply(&rx), ("/a/ok/play".into(), "started".into(), "".into()));

    // 2) 파일 없음: ack → rejected file_missing
    send("/a/gone/play");
    assert_eq!(reply(&rx), ("/a/gone/play".into(), "ack".into(), "".into()));
    assert_eq!(reply(&rx), ("/a/gone/play".into(), "rejected".into(), "file_missing".into()));

    // 3) 활성 방이 아닌 방: rejected gated
    send("/b/x/play");
    assert_eq!(reply(&rx), ("/b/x/play".into(), "rejected".into(), "gated".into()));

    // 4) 등록되지 않은 주소: rejected unregistered
    send("/nowhere");
    assert_eq!(reply(&rx), ("/nowhere".into(), "rejected".into(), "unregistered".into()));

    // 5) 상태 조회: 스냅샷(재생 중 a/ok)
    send("/awesome/query/status");
    let snap = recv(&rx);
    assert_eq!(snap.addr, "/awesome/status/snapshot");
    assert_eq!(snap.args[0], OscType::String("Running".into()));
    assert_eq!(snap.args[2], OscType::Int(1));
    assert_eq!(snap.args[3], OscType::String("a/ok".into()));

    // 6) 엔진이 꺼진 동안의 재생: 동작은 그대로(명령은 넘김), ack만 오고 started는 오지 않는다.
    ENGINE_ACTIVE.store(false, Ordering::SeqCst);
    GLOBAL_STATE.clear_playing_tracks();
    std::thread::sleep(Duration::from_millis(300)); // 같은 주소 디바운스(250ms)를 넘긴다
    send("/a/ok/play");
    assert_eq!(reply(&rx), ("/a/ok/play".into(), "ack".into(), "".into()));
    rx.set_read_timeout(Some(Duration::from_millis(500))).unwrap();
    let mut buf = [0u8; 512];
    assert!(rx.recv_from(&mut buf).is_err(), "엔진이 꺼져 있으면 started가 오면 안 된다");
    rx.set_read_timeout(Some(Duration::from_secs(3))).unwrap();

    // 7) 방 클리어(규격 §3.4): 응답은 OSC 쪽(listener)에서만 보내고, 본체는 대시보드와 같은 공통 함수다.
    //    활성 방이 아닌 방의 클리어 → rejected gated
    send("/b/clear");
    assert_eq!(reply(&rx), ("/b/clear".into(), "rejected".into(), "gated".into()));

    //    활성 방의 클리어 → ack(본체 실행 전에 보냄). 다음 방(b)이 활성이 된다.
    send("/a/clear");
    assert_eq!(reply(&rx), ("/a/clear".into(), "ack".into(), "".into()));
    let end = std::time::Instant::now() + Duration::from_secs(2);
    while GLOBAL_STATE.active_room_id.read().unwrap().as_deref() != Some("b") {
        assert!(std::time::Instant::now() < end, "클리어 뒤 다음 방이 활성이 되어야 한다");
        std::thread::sleep(Duration::from_millis(20));
    }

    //    전시 모드에서는 클리어를 무시한다 → rejected exhibition_ignored
    {
        let mut guard = GLOBAL_STATE.config.write().unwrap();
        if let Some(c) = guard.as_mut() {
            c.is_exhibition_mode = true;
        }
    }
    GLOBAL_STATE.config_version.fetch_add(1, Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(300)); // 같은 주소 디바운스(250ms)를 넘긴다
    send("/b/clear");
    assert_eq!(reply(&rx), ("/b/clear".into(), "rejected".into(), "exhibition_ignored".into()));
}
