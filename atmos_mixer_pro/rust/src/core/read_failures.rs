//! 디스크 스트리밍이 음원 파일을 다시 열지 못해 무음으로 기다리는 트랙 목록(검토 A6).
//!
//! 디코더 스레드(`audio::streaming`)가 다시 열기에 실패하면 [`begin`]으로 등록하고, 회복하거나 정지되면(토큰을
//! 버리면) 빠진다. 목록이 바뀔 때마다 상태 스트림으로 알린다(대시보드 배너, Dart `read_failing_banner.dart`).
//!
//! 상태 스트림 값 계약: `ReadFailing:<트랙 수>:<첫 파일 이름>`(수가 1 이상), 모두 회복·정지되면 `ReadFailing:0`.
//! 파일 이름은 경로가 아니라 마지막 이름이고, 이름에 ':'가 있을 수 있어 맨 뒤에 둔다(앞 두 칸만 나눠 읽는다).
//! "첫 파일"은 지금 실패 중인 것 가운데 가장 먼저 실패한 것이다. 엔진 상태가 아니다.
//!
//! 엔진을 다시 열면 옛 인스턴스와 함께 스트리머가 버려져 토큰이 빠지므로 목록은 엔진 상태와 따로 맞는다
//! (배너는 `Running`이 아니라 이 목록만 따른다). 실시간 콜백에서는 부르지 않는다(Mutex·할당).
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// 상태 스트림 접두사.
pub const READ_FAILING_PREFIX: &str = "ReadFailing:";
/// 같은 파일의 실패 사건 로그는 이 간격에 한 번만 남긴다.
pub const FAILED_LOG_INTERVAL: Duration = Duration::from_secs(60);

struct Failing {
    id: u64,
    file_name: String,
}

static FAILING: Mutex<Vec<Failing>> = Mutex::new(Vec::new());
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static LAST_FAILED_LOG: Mutex<Option<HashMap<String, Instant>>> = Mutex::new(None);

/// 실패 중인 트랙 하나. 버리면(회복·정지·스레드 끝) 목록에서 빠지고 상태를 다시 알린다.
pub struct FailureToken {
    id: u64,
}

impl Drop for FailureToken {
    fn drop(&mut self) {
        // 잠금을 쥔 채 알린다: 밖에서 알리면 동시에 들어온 begin·drop의 옛 값이 마지막에 나갈 수 있다(검토 R4).
        let mut list = FAILING.lock().unwrap_or_else(|e| e.into_inner());
        list.retain(|f| f.id != self.id);
        crate::api::simple::broadcast_stream_status(status_of(&list));
    }
}

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

fn status_of(list: &[Failing]) -> String {
    match list.first() {
        Some(first) => format!("{READ_FAILING_PREFIX}{}:{}", list.len(), first.file_name),
        None => format!("{READ_FAILING_PREFIX}0"),
    }
}

/// `path`를 다시 열지 못해 기다리기 시작한다. 목록에 넣고 상태를 알린다.
pub fn begin(path: &Path) -> FailureToken {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    // 잠금을 쥔 채 알린다(Drop과 같은 이유, 검토 R4).
    let mut list = FAILING.lock().unwrap_or_else(|e| e.into_inner());
    list.push(Failing { id, file_name: file_name_of(path) });
    crate::api::simple::broadcast_stream_status(status_of(&list));
    drop(list);
    FailureToken { id }
}

/// 지금 상태 문자열(`ReadFailing:0` 포함).
pub fn status() -> String {
    status_of(&FAILING.lock().unwrap_or_else(|e| e.into_inner()))
}

/// 실패 중인 트랙이 있으면 그 상태 문자열. 새로 구독한 쪽에 처음 값으로 보낸다(`api_create_stream_status_stream`).
pub fn current_status() -> Option<String> {
    let list = FAILING.lock().unwrap_or_else(|e| e.into_inner());
    if list.is_empty() {
        None
    } else {
        Some(status_of(&list))
    }
}

/// 실패 중인 트랙 수.
pub fn failing_count() -> usize {
    FAILING.lock().unwrap_or_else(|e| e.into_inner()).len()
}

/// 같은 파일의 실패 사건을 지금 남겨도 되는가(파일마다 [`FAILED_LOG_INTERVAL`]에 한 번). 남기면 시각을 적는다.
pub fn should_log_failure(path: &Path) -> bool {
    let key = path.to_string_lossy().to_string();
    let mut guard = LAST_FAILED_LOG.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    let now = Instant::now();
    match map.get(&key) {
        Some(at) if now.duration_since(*at) < FAILED_LOG_INTERVAL => false,
        _ => {
            map.insert(key, now);
            true
        }
    }
}
