//! OSC 인자 없는 메시지 트리거 시험(작업 A, 규격 §2, 2026-11-12 확정).
//! - 순수 판정(`is_trigger`): 주소만 · 숫자 · 문자열 · 잘못된 인자(거짓·0)
//! - 실제 리스너 경로: 실제 OSC 리스너에 UDP로 보내 트랙이 틀리는지 본다(디바운스·라우터 포함).
//!   전역 상태를 쓰므로 이 파일의 시험은 하나다.
use rosc::{encoder, OscMessage, OscPacket, OscType};
use rust_lib_atmos_mixer_pro::api::simple::{api_set_active_room, api_start_osc_listener};
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use rust_lib_atmos_mixer_pro::osc::listener::is_trigger;
use std::net::UdpSocket;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

// ---- 순수 판정 ----

#[test]
fn 주소만_있는_메시지는_트리거다() {
    assert!(is_trigger(&[]));
}

#[test]
fn 숫자_인자는_양수일_때만_트리거다() {
    assert!(is_trigger(&[OscType::Int(1)]));
    assert!(is_trigger(&[OscType::Float(0.5)]));
    assert!(
        !is_trigger(&[OscType::Int(0)]),
        "Int 0은 버튼을 뗀 값이라 무시한다"
    );
    assert!(
        !is_trigger(&[OscType::Float(0.0)]),
        "Float 0은 버튼을 뗀 값이라 무시한다"
    );
    assert!(
        !is_trigger(&[OscType::Int(-1)]),
        "음수는 트리거 조건(> 0)이 아니다"
    );
}

#[test]
fn 문자열_인자는_트리거다() {
    assert!(is_trigger(&[OscType::String("go".into())]));
}

#[test]
fn 거짓_인자는_무시하고_참_인자는_트리거다() {
    assert!(
        !is_trigger(&[OscType::Bool(false)]),
        "Bool false는 무시한다(작업 A)"
    );
    assert!(is_trigger(&[OscType::Bool(true)]));
}

#[test]
fn 여러_인자는_하나라도_트리거_조건이면_트리거다() {
    // 기존 규칙 그대로: 여러 인자 중 하나라도 트리거 조건이면 트리거다.
    assert!(is_trigger(&[
        OscType::Float(0.0),
        OscType::String("x".into())
    ]));
    assert!(!is_trigger(&[OscType::Int(0), OscType::Bool(false)]));
}

// ---- 실제 리스너 경로 ----

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

fn playing() -> Vec<String> {
    let mut v: Vec<String> = GLOBAL_STATE
        .playing_track_ids
        .read()
        .unwrap()
        .values()
        .cloned()
        .collect();
    v.sort();
    v
}

fn wait_until_playing(id: &str) {
    let end = Instant::now() + Duration::from_secs(2);
    while !playing().iter().any(|t| t == id) {
        assert!(
            Instant::now() < end,
            "{id}: 트리거여야 하는데 재생되지 않았다 (재생 목록 {:?})",
            playing()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn 실제_리스너는_주소만_숫자_문자열_메시지는_틀고_거짓과_0은_무시한다() {
    let dir = std::env::temp_dir().join(format!("atmos_osc_trigger_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let names = ["la", "sa", "ta", "fa", "za"];
    for n in names {
        tone_wav(&dir.join(format!("{n}.wav")), 3.0);
    }
    let p = |n: &str| dir.join(format!("{n}.wav")).to_string_lossy().into_owned();
    GLOBAL_STATE
        .engine_sample_rate
        .store(48_000, Ordering::SeqCst);

    let port = UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port(); // 빈 포트
    let config = AppConfig {
        osc_port: port,
        is_exhibition_mode: false,
        rooms: vec![RoomConfig {
            id: "a".into(),
            name: "a".into(),
            color_hex: "#ffffff".into(),
            volume: 1.0,
            clear_osc_address: "/a/clear".into(),
            tracks: names.iter().map(|n| track("a", n, &p(n))).collect(),
        }],
        ..AppConfig::default()
    };
    *GLOBAL_STATE.config.write().unwrap() = Some(config);
    GLOBAL_STATE.config_version.fetch_add(1, Ordering::SeqCst); // OSC 라우터 캐시를 새로 만들게 한다
    GLOBAL_STATE.clear_playing_tracks();
    api_start_osc_listener(port);
    std::thread::sleep(Duration::from_millis(300)); // 리스너 스레드가 포트를 잡을 때까지
    api_set_active_room(Some("a".into())).unwrap();

    let sock = UdpSocket::bind("127.0.0.1:0").unwrap();
    let send = |addr: &str, args: Vec<OscType>| {
        let msg = OscPacket::Message(OscMessage {
            addr: addr.into(),
            args,
        });
        sock.send_to(&encoder::encode(&msg).unwrap(), ("127.0.0.1", port))
            .unwrap();
    };

    // 1) 주소만 있는 메시지 → 트리거(작업 A 수정 전에는 무시되었다)
    send("/a/la/play", vec![]);
    wait_until_playing("la");

    // 2) 숫자 1 → 트리거
    send("/a/sa/play", vec![OscType::Int(1)]);
    wait_until_playing("sa");

    // 3) 문자열 인자 → 트리거(규격 §2 "그 밖의 형식")
    send("/a/ta/play", vec![OscType::String("go".into())]);
    wait_until_playing("ta");

    // 4) 잘못된 인자: 거짓과 0은 트리거가 아니다. 충분히 기다린 뒤 재생 목록에 없어야 한다.
    send("/a/fa/play", vec![OscType::Bool(false)]);
    send("/a/za/play", vec![OscType::Float(0.0)]);
    std::thread::sleep(Duration::from_millis(400));
    assert!(
        !playing().iter().any(|t| t == "fa"),
        "Bool false로 트리거되었다"
    );
    assert!(
        !playing().iter().any(|t| t == "za"),
        "Float 0으로 트리거되었다"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
