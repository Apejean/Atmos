//! OSC 재생·방 비우기 경로. 실제 OSC 리스너에 UDP로 보낸다. 전역 상태를 쓰므로 테스트는 하나다.
//! - OSC 재생은 대시보드와 같은 경로라 루프·스트리밍 트랙도 나온다.
//! - 방 비우기(OSC·대시보드)는 그 방 트랙만 재생 목록에서 빼고, OSC로 다음 방에 넘어가면 그 방 BGM(루프)을 튼다.
//! - OSC 테마 시작은 전체 정지 뒤 첫 방을 활성으로 하고 그 방 루프만 튼다(감시 재실행과 같은 함수).
use rosc::{encoder, OscMessage, OscPacket, OscType};
use rust_lib_atmos_mixer_pro::api::simple::{
    api_clear_room, api_play_track, api_set_active_room, api_start_osc_listener,
};
use rust_lib_atmos_mixer_pro::audio::player::SoundData;
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::net::UdpSocket;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

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

fn track(room: &str, id: &str, path: &str, is_loop: bool, is_streaming: bool) -> TrackConfig {
    TrackConfig {
        id: id.into(),
        name: id.into(),
        file_path: path.into(),
        volume: 1.0,
        is_loop,
        is_streaming,
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
        volume_osc_address: String::new(),
        clear_osc_address: format!("/{id}/clear"),
        tracks,
    }
}

fn playing() -> Vec<String> {
    let mut v: Vec<String> = GLOBAL_STATE.playing_track_ids.read().unwrap().values().cloned().collect();
    v.sort();
    v
}

/// 큐에 쌓인 명령을 비우고, 그중 재생 명령의 트랙 이름을 돌려준다.
fn drained_plays() -> Vec<String> {
    let mut v = Vec::new();
    while let Ok(c) = GLOBAL_STATE.command_receiver.try_recv() {
        if let AudioCommand::PlayTrack { instance, .. } = c {
            v.push(instance.track_id_str.clone());
        }
    }
    v
}

fn wait_until(what: &str, cond: impl Fn() -> bool) {
    let end = Instant::now() + Duration::from_secs(2);
    while !cond() {
        assert!(
            Instant::now() < end,
            "{what} (재생 목록 {:?}, 활성 방 {:?})",
            playing(),
            GLOBAL_STATE.active_room_id.read().unwrap()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn osc_재생은_루프와_스트리밍도_틀고_방_비우기는_그_방만_빼고_다음_방_bgm을_튼다() {
    let dir = std::env::temp_dir().join(format!("atmos_osc_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for name in ["la", "sa", "lb"] {
        tone_wav(&dir.join(format!("{name}.wav")), 3.0);
    }
    let p = |n: &str| dir.join(format!("{n}.wav")).to_string_lossy().into_owned();
    // 미리 로드한 단발(oa)은 api_preload_all_sounds처럼 sound_cache에 올라가 있다.
    GLOBAL_STATE.sound_cache.write().unwrap().insert(
        "/oa.wav".into(),
        Arc::new(SoundData { samples: vec![0.1; 48_000], channels: 1, sample_rate: 48_000 }),
    );
    GLOBAL_STATE.engine_sample_rate.store(48_000, Ordering::SeqCst);

    let port = UdpSocket::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port(); // 빈 포트
    let config = AppConfig {
        osc_port: port,
        is_exhibition_mode: true,
        theme_start_osc_address: "/theme/start".into(),
        rooms: vec![
            room(
                "a",
                vec![
                    track("a", "la", &p("la"), true, false),
                    track("a", "sa", &p("sa"), false, true),
                    track("a", "oa", "/oa.wav", false, false),
                ],
            ),
            room("b", vec![track("b", "lb", &p("lb"), true, false)]),
        ],
        ..AppConfig::default()
    };
    *GLOBAL_STATE.config.write().unwrap() = Some(config);
    GLOBAL_STATE.config_version.fetch_add(1, Ordering::SeqCst); // OSC 라우터 캐시를 새로 만들게 한다
    GLOBAL_STATE.clear_playing_tracks();
    api_start_osc_listener(port);
    std::thread::sleep(Duration::from_millis(300)); // 리스너 스레드가 포트를 잡을 때까지

    let sock = UdpSocket::bind("127.0.0.1:0").unwrap();
    let send = |addr: &str| {
        let msg = OscPacket::Message(OscMessage { addr: addr.into(), args: vec![OscType::Int(1)] });
        sock.send_to(&encoder::encode(&msg).unwrap(), ("127.0.0.1", port)).unwrap();
    };
    let has = |id: &str| playing().iter().any(|t| t == id);

    // 1) OSC 재생: 루프·스트리밍·미리 로드 단발 모두 재생된다
    send("/a/la/play");
    wait_until("OSC로 루프 트랙을 틀었는데 재생되지 않았다", || has("la"));
    send("/a/sa/play");
    wait_until("OSC로 스트리밍 트랙을 틀었는데 재생되지 않았다", || has("sa"));
    send("/a/oa/play");
    wait_until("OSC로 미리 로드한 단발을 틀었는데 재생되지 않았다", || has("oa"));

    // 2) 방 B BGM도 함께 돈다(대시보드 Start처럼 여러 방이 동시에 도는 상태)
    api_play_track("b".into(), "lb".into()).unwrap();

    // 3) OSC 방 A 비우기: 방 A 트랙만 빠지고 방 B BGM은 그대로(겹쳐 다시 틀지 않음), 방 B가 활성
    drained_plays();
    send("/a/clear");
    wait_until("OSC로 방 A를 비웠는데 방 B가 활성이 되지 않았다", || {
        GLOBAL_STATE.active_room_id.read().unwrap().as_deref() == Some("b")
    });
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(playing(), vec!["lb".to_string()], "OSC로 방 A를 비운 뒤 재생 목록(방 B BGM만 한 번)");
    // 목록만 보면 모른다: 목록에서 지워진 BGM을 다시 틀면 엔진에서는 옛 인스턴스와 겹쳐 두 번 나온다
    assert!(drained_plays().is_empty(), "방 A를 비울 때 이미 돌던 방 B BGM을 또 틀었다(겹쳐 두 번 나온다)");

    // 4) 대시보드 방 비우기도 그 방 트랙만 뺀다
    api_play_track("a".into(), "la".into()).unwrap();
    api_set_active_room(Some("b".into())).unwrap();
    api_clear_room("b".into()).unwrap();
    assert_eq!(playing(), vec!["la".to_string()], "대시보드로 방 B를 비운 뒤 재생 목록");

    // 5) OSC 방 비우기로 다음 방에 넘어가면 그 방 BGM(루프)이 자동으로 나온다
    GLOBAL_STATE.clear_playing_tracks();
    std::thread::sleep(Duration::from_millis(300)); // 같은 주소를 250ms 안에 다시 받으면 무시한다
    send("/a/clear");
    wait_until("OSC로 방 A를 비웠는데 다음 방 B의 BGM이 자동으로 나오지 않았다", || has("lb"));

    // 6) OSC 테마 시작: 전체 정지 뒤 첫 방(A)을 활성으로 하고 그 방 루프만 튼다
    send("/theme/start");
    wait_until("OSC 테마 시작 뒤 첫 방 A가 활성이고 그 방 루프만 돌아야 한다", || {
        GLOBAL_STATE.active_room_id.read().unwrap().as_deref() == Some("a") && playing() == vec!["la".to_string()]
    });

    drained_plays();
    let _ = std::fs::remove_dir_all(&dir);
}
