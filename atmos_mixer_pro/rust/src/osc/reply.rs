//! OSC 응답·시작 신호·상태 보고 송신(docs/03_Protocols_and_Workflows/OSC_INTERFACE_SPEC.md §3).
//!
//! - 목적지는 설정 `osc_reply_targets`("IP:포트" 목록)에서 보낼 때마다 읽는다. 빈 목록이면 보내지 않는다.
//! - 보내는 소켓은 받는 소켓과 따로 만든다(Windows UDP `WSAECONNRESET`이 수신을 끊지 않게).
//! - 송신은 전용 스레드 하나가 한다. 부르는 쪽은 bounded 채널에 `try_send`만 하므로 막히지 않고,
//!   가득 차면 버리고 카운터를 올린다. 오디오 콜백에서는 부르지 않는다(할당).
//! - 채널의 송신단이 모두 사라지면(앱 종료, [`shutdown`]) 송신 스레드는 스스로 끝난다.

use crossbeam_channel::{bounded, Receiver, Sender, TrySendError};
use rosc::{OscMessage, OscPacket, OscType};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// 목적지 최대 개수(규격 §3.1).
pub const MAX_TARGETS: usize = 8;
/// 송신 대기열 길이.
const QUEUE_LEN: usize = 256;
/// 같은 종류의 송신 실패·잘못된 목적지 로그 간격.
const FAIL_LOG_INTERVAL: Duration = Duration::from_secs(60);
/// 시작 신호 반복 횟수와 간격(UDP라 여러 번 보내고 받는 쪽이 boot id로 거른다).
pub const BOOT_REPEAT: u32 = 3;
const BOOT_INTERVAL: Duration = Duration::from_secs(1);

pub const ADDR_REPLY: &str = "/awesome/reply";
pub const ADDR_BOOT: &str = "/awesome/status/boot";
pub const ADDR_SNAPSHOT: &str = "/awesome/status/snapshot";
pub const ADDR_QUERY_STATUS: &str = "/awesome/query/status";
/// 스냅샷 트랙 목록 문자열 상한(바이트). UDP 한 패킷(약 1.4KB)을 넘지 않게 자른다.
pub const SNAPSHOT_LIST_MAX_BYTES: usize = 1200;

/// 명령 응답 단계(규격 §3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplyStage {
    Ack,
    Started,
    Rejected,
}

impl ReplyStage {
    pub fn as_str(self) -> &'static str {
        match self {
            ReplyStage::Ack => "ack",
            ReplyStage::Started => "started",
            ReplyStage::Rejected => "rejected",
        }
    }
}

/// 거절 이유 코드(규격 §3.4).
pub mod reason {
    pub const UNREGISTERED: &str = "unregistered";
    pub const GATED: &str = "gated";
    pub const ENGINE_WAITING: &str = "engine_waiting";
    pub const FILE_MISSING: &str = "file_missing";
    pub const EXHIBITION_IGNORED: &str = "exhibition_ignored";
    /// 파일 없음 밖의 재생 실패(스트리머 열기 실패 등).
    pub const PLAY_FAILED: &str = "play_failed";
}

/// 재생 함수(api::simple::play_track_from)가 파일을 못 찾았을 때 돌려주는 오류 문구의 앞부분.
pub const FILE_MISSING_PREFIX: &str = "파일을 찾을 수 없습니다";

/// 엔진이 장치를 기다리거나 실패 상태라 재생 명령을 버릴 때의 오류 문구 앞부분(core::engine_keeper::accepts_play).
pub const ENGINE_WAITING_PREFIX: &str = "엔진이 오디오 장치를 기다리는 중이라 재생하지 않습니다";

/// 재생 실패 오류를 거절 이유 코드로 바꾼다.
pub fn reason_for_play_error(message: &str) -> &'static str {
    if message.starts_with(FILE_MISSING_PREFIX) {
        reason::FILE_MISSING
    } else if message.starts_with(ENGINE_WAITING_PREFIX) {
        reason::ENGINE_WAITING
    } else {
        reason::PLAY_FAILED
    }
}

/// 목적지 하나를 검사한다. 거부하면 이유를 돌려준다.
/// 미지정 주소(0.0.0.0·::), 포트 0, 멀티캐스트, 255.255.255.255는 받지 않는다. 서브넷 브로드캐스트는
/// 서브넷을 모르므로 여기서 막지 않는다(규격에 "넣지 말 것"으로 적는다).
pub fn check_target(text: &str) -> Result<SocketAddr, &'static str> {
    let addr: SocketAddr = text.trim().parse().map_err(|_| "not_ip_port")?;
    if addr.port() == 0 {
        return Err("port_zero");
    }
    let ip = addr.ip();
    if ip.is_unspecified() {
        return Err("unspecified");
    }
    if ip.is_multicast() {
        return Err("multicast");
    }
    if ip == IpAddr::V4(Ipv4Addr::BROADCAST) {
        return Err("broadcast");
    }
    Ok(addr)
}

/// 설정의 목적지 목록을 검사한다. 앞에서부터 [`MAX_TARGETS`]개까지 쓰고, 나머지와 잘못된 항목은
/// `(원문, 이유)`로 돌려준다. 설정 원문은 바꾸지 않는다(실행할 때만 건너뛴다).
pub fn parse_targets(list: &[String]) -> (Vec<SocketAddr>, Vec<(String, &'static str)>) {
    let mut ok = Vec::new();
    let mut bad = Vec::new();
    for item in list {
        if item.trim().is_empty() {
            continue;
        }
        match check_target(item) {
            Ok(addr) if ok.len() < MAX_TARGETS => {
                if !ok.contains(&addr) {
                    ok.push(addr);
                }
            }
            Ok(_) => bad.push((item.clone(), "over_limit")),
            Err(why) => bad.push((item.clone(), why)),
        }
    }
    (ok, bad)
}

// ---- 메시지 구성(순수 함수, 시험 대상) ----

pub fn reply_message(addr: &str, stage: ReplyStage, reason: &str) -> OscMessage {
    OscMessage {
        addr: ADDR_REPLY.to_string(),
        args: vec![
            OscType::String(addr.to_string()),
            OscType::String(stage.as_str().to_string()),
            OscType::String(reason.to_string()),
        ],
    }
}

/// 시작 종류. 모르는 값은 `manual`로 본다(규격 §3.2, 부르는 쪽이 로그를 남긴다).
pub fn normalize_start_kind(kind: &str) -> (&'static str, bool) {
    match kind {
        "manual" => ("manual", true),
        "logon" => ("logon", true),
        "relaunch" => ("relaunch", true),
        _ => ("manual", false),
    }
}

pub struct BootInfo<'a> {
    pub version: &'a str,
    pub engine_state: &'a str,
    pub device: &'a str,
    pub start_kind: &'a str,
    pub resumed_tracks: i32,
    pub boot_id: &'a str,
}

pub fn boot_message(b: &BootInfo) -> OscMessage {
    OscMessage {
        addr: ADDR_BOOT.to_string(),
        args: vec![
            OscType::String(b.version.to_string()),
            OscType::String(b.engine_state.to_string()),
            OscType::String(b.device.to_string()),
            OscType::String(b.start_kind.to_string()),
            OscType::Int(b.resumed_tracks),
            OscType::String(b.boot_id.to_string()),
        ],
    }
}

/// 재생 중 트랙 목록을 쉼표로 잇되 [`SNAPSHOT_LIST_MAX_BYTES`]를 넘으면 자르고 `,…`를 붙인다.
pub fn join_track_list(ids: &[String]) -> String {
    let mut out = String::new();
    for (i, id) in ids.iter().enumerate() {
        let extra = if i == 0 { id.len() } else { id.len() + 1 };
        if out.len() + extra > SNAPSHOT_LIST_MAX_BYTES {
            out.push_str(",…");
            return out;
        }
        if i > 0 {
            out.push(',');
        }
        out.push_str(id);
    }
    out
}

pub fn snapshot_message(
    engine_state: &str,
    cause: &str,
    playing: &[String],
    processing_channels: i32,
) -> OscMessage {
    OscMessage {
        addr: ADDR_SNAPSHOT.to_string(),
        args: vec![
            OscType::String(engine_state.to_string()),
            OscType::String(cause.to_string()),
            OscType::Int(playing.len().min(i32::MAX as usize) as i32),
            OscType::String(join_track_list(playing)),
            OscType::Int(processing_channels),
        ],
    }
}

// ---- 송신 스레드 ----

static SENDER: Mutex<Option<Sender<OscMessage>>> = Mutex::new(None);
/// [`shutdown`] 뒤에는 송신 스레드를 다시 띄우지 않는다(종료 절차 중 응답이 스레드를 되살리지 않게).
static CLOSED: AtomicBool = AtomicBool::new(false);
/// 대기열이 가득 차 버린 메시지 수.
pub static DROPPED: AtomicU32 = AtomicU32::new(0);

fn sender() -> Option<Sender<OscMessage>> {
    let mut guard = SENDER.lock().unwrap_or_else(|e| e.into_inner());
    if CLOSED.load(Ordering::SeqCst) {
        return None;
    }
    if guard.is_none() {
        let (tx, rx) = bounded::<OscMessage>(QUEUE_LEN);
        let spawned = std::thread::Builder::new()
            .name("osc-reply".to_string())
            .spawn(move || run_sender(rx));
        if spawned.is_err() {
            crate::core::state::GLOBAL_STATE.log("OSC 응답 송신 스레드를 만들지 못했다".to_string());
            return None;
        }
        *guard = Some(tx);
    }
    guard.clone()
}

/// 송신 대기열을 닫는다. 송신 스레드는 남은 메시지를 보내고 끝나며, 그 뒤 [`send`]는 아무것도 하지 않는다.
pub fn shutdown() {
    let mut guard = SENDER.lock().unwrap_or_else(|e| e.into_inner());
    CLOSED.store(true, Ordering::SeqCst);
    guard.take();
}

/// 송신이 닫혔는지(시험용).
pub fn is_closed() -> bool {
    CLOSED.load(Ordering::SeqCst)
}

fn configured_targets() -> Vec<String> {
    crate::core::state::GLOBAL_STATE
        .config
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .map(|c| c.osc_reply_targets.clone())
        .unwrap_or_default()
}

/// 메시지 하나를 대기열에 넣는다. 목적지가 없으면 아무것도 하지 않는다.
pub fn send(msg: OscMessage) {
    if configured_targets().iter().all(|t| t.trim().is_empty()) {
        return;
    }
    let Some(tx) = sender() else { return };
    match tx.try_send(msg) {
        Ok(()) => {}
        Err(TrySendError::Full(_)) => {
            DROPPED.fetch_add(1, Ordering::Relaxed);
        }
        Err(TrySendError::Disconnected(_)) => {}
    }
}

pub fn send_reply(addr: &str, stage: ReplyStage, reason: &str) {
    send(reply_message(addr, stage, reason));
}

fn run_sender(rx: Receiver<OscMessage>) {
    let mut v4: Option<UdpSocket> = None;
    let mut v6: Option<UdpSocket> = None;
    let mut last_fail_log: Option<Instant> = None;
    let mut last_bad_log: Option<Instant> = None;
    let due = |t: &Option<Instant>| t.map_or(true, |t| t.elapsed() >= FAIL_LOG_INTERVAL);

    while let Ok(msg) = rx.recv() {
        let (targets, bad) = parse_targets(&configured_targets());
        if !bad.is_empty() && due(&last_bad_log) {
            let list: Vec<String> = bad.iter().map(|(t, why)| format!("{t}({why})")).collect();
            crate::core::event_log::log_event("osc_reply_target_skipped", &[("items", &list.join(","))]);
            last_bad_log = Some(Instant::now());
        }
        if targets.is_empty() {
            continue;
        }
        let bytes = match rosc::encoder::encode(&OscPacket::Message(msg)) {
            Ok(b) => b,
            Err(_) => continue,
        };
        for dest in targets {
            let slot = if dest.is_ipv4() { &mut v4 } else { &mut v6 };
            if slot.is_none() {
                let bind = if dest.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };
                *slot = UdpSocket::bind(bind).ok();
            }
            let result = match slot.as_ref() {
                Some(sock) => sock.send_to(&bytes, dest).map(|_| ()),
                None => Err(std::io::Error::other("send socket bind failed")),
            };
            if let Err(e) = result {
                // 소켓이 망가졌을 수 있으니 다음 송신에서 다시 만든다.
                *slot = None;
                if due(&last_fail_log) {
                    crate::core::event_log::log_event(
                        "osc_send_failed",
                        &[("dest", &dest.to_string()), ("error", &e.to_string())],
                    );
                    last_fail_log = Some(Instant::now());
                }
            }
        }
    }
}

// ---- 시작 신호 ----

static BOOT_SENT: AtomicBool = AtomicBool::new(false);
/// 이어 재생한 트랙 수. 이어 가기(core::show_state)가 끝난 뒤 정한다.
pub static RESUMED_TRACKS: AtomicU32 = AtomicU32::new(0);
static BOOT_ID_MS: AtomicU64 = AtomicU64::new(0);

/// 시작 신호를 한 번만 보낸다(같은 실행에서 두 번 불러도 한 번, 그 안에서 1초 간격 3회).
/// 처음 부른 것이면 true.
pub fn announce_boot(start_kind: &str, app_version: &str) -> bool {
    if BOOT_SENT.swap(true, Ordering::SeqCst) {
        return false;
    }
    let (kind, known) = normalize_start_kind(start_kind);
    if !known {
        crate::core::state::GLOBAL_STATE.log(format!(
            "OSC 시작 신호: 모르는 시작 종류 '{start_kind}'를 manual로 본다"
        ));
    }
    let boot_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    BOOT_ID_MS.store(boot_ms, Ordering::Relaxed);
    let boot_id = boot_ms.to_string();
    // 버전을 못 받았으면 "unknown"으로 보낸다.
    let version = if app_version.trim().is_empty() { "unknown".to_string() } else { app_version.trim().to_string() };
    let kind = kind.to_string();
    let spawned = std::thread::Builder::new().name("osc-boot".to_string()).spawn(move || {
        for i in 0..BOOT_REPEAT {
            if i > 0 {
                std::thread::sleep(BOOT_INTERVAL);
            }
            let (engine_state, device) = engine_state_and_device();
            send(boot_message(&BootInfo {
                version: &version,
                engine_state,
                device: &device,
                start_kind: &kind,
                resumed_tracks: RESUMED_TRACKS.load(Ordering::Relaxed).min(i32::MAX as u32) as i32,
                boot_id: &boot_id,
            }));
        }
    });
    if spawned.is_err() {
        crate::core::state::GLOBAL_STATE.log("OSC 시작 신호 스레드를 만들지 못했다".to_string());
    }
    true
}

/// 엔진 상태(규격의 값, 엔진 유지자 core::engine_keeper)와 장치 이름. 장치 이름은 엔진이 돌 때만 설정값(대시보드
/// 머리말과 같음)을 싣는다. 비상 전환(기본 장치) 중이면 설정 장치가 아니므로 빈 문자열이다.
pub fn engine_state_and_device() -> (&'static str, String) {
    let state = crate::core::engine_keeper::state();
    let failover = crate::core::state::GLOBAL_STATE.is_failover_mode.load(Ordering::Relaxed);
    let device = if state == crate::core::engine_keeper::EngineState::Running && !failover {
        crate::core::state::GLOBAL_STATE
            .config
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .and_then(|c| c.device_name.clone())
            .unwrap_or_default()
    } else {
        String::new()
    };
    (state.name(), device)
}

// ---- 상태 조회 ----

/// 재생 중 트랙을 `방id/트랙id`로(이름 순, 같은 트랙은 한 번). 방을 못 찾으면 트랙 id만 쓴다.
pub fn playing_track_labels() -> Vec<String> {
    let state = &crate::core::state::GLOBAL_STATE;
    let mut track_ids: Vec<String> = state
        .playing_track_ids
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .values()
        .cloned()
        .collect();
    track_ids.sort();
    track_ids.dedup();
    let config = state.config.read().unwrap_or_else(|e| e.into_inner());
    track_ids
        .into_iter()
        .map(|track_id| {
            let room = config.as_ref().and_then(|c| {
                c.rooms.iter().find(|r| r.tracks.iter().any(|t| t.id == track_id))
            });
            match room {
                Some(r) => format!("{}/{}", r.id, track_id),
                None => track_id,
            }
        })
        .collect()
}

/// `/awesome/query/status`에 답한다(`/awesome/status/snapshot`, 규격 §3.6). 목적지 목록으로만 간다.
pub fn send_status_snapshot() {
    let (engine_state, _) = engine_state_and_device();
    let channels = crate::core::state::GLOBAL_STATE
        .output_processing_channels
        .load(Ordering::Relaxed)
        .min(i32::MAX as u32) as i32;
    send(snapshot_message(engine_state, "", &playing_track_labels(), channels));
}
