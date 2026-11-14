//! 엔진 수명을 맡는 유지자(안정성 ③, 설계 `docs/superpowers/specs/2026-11-14-engine-keeper-design.md`).
//!
//! 예전에는 자기 재시작(워치독·장치 오류·장치 목록 변화)의 재연결이 일회성 스레드 하나였다. 설정 장치와 기본 장치
//! (비상 전환)가 모두 실패하면 아무도 다시 시도하지 않아, 사람이 알아차리기 전까지 무음이었다. 이제 이 모듈의 스레드
//! 하나가 다시 열기를 맡고, 장치가 없으면 1·2·4·8·16·30초, 그 뒤 30초마다 계속 찾는다(사용자 결정 U1).
//!
//! 상태값은 `Running / WaitingForDevice / Failed / StoppedByUser`(OSC 규격 §3.2와 같은 이름)이다. 하트비트·상태 방송·
//! OSC가 이 값을 쓴다. 오디오 콜백은 이 모듈을 보지 않는다(3법칙과 무관).
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard, RwLock};
use std::time::{Duration, Instant};

use crate::core::state::GLOBAL_STATE;

/// 실패 원인.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailCause {
    /// 엔진 시작 호출이 돌아오지 않았다(ASIO 드라이버 멈춤). 같은 드라이버에서 또 멈추므로 더 시도하지 않는다.
    StartHung,
    /// 장치는 찾았는데 열기가 연속으로 실패했다(사용자 결정 U3: 3번). 30초마다 계속 시도한다.
    OpenFailed,
    /// 이 PC에서 나타날 수 없는 장치다(다른 OS의 이름, 설치되지 않은 ASIO 드라이버). 시도하지 않는다.
    Unavailable,
}

impl FailCause {
    /// 사건 로그·배너(`EngineDown:<원인>`)·OSC에 쓰는 짧은 원인.
    pub fn as_str(self) -> &'static str {
        match self {
            FailCause::StartHung => "start_hung",
            FailCause::OpenFailed => "open_failed",
            FailCause::Unavailable => "unavailable",
        }
    }
}

/// 엔진 상태값(OSC 규격 §3.2·§3.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineState {
    Running,
    WaitingForDevice,
    Failed(FailCause),
    StoppedByUser,
}

impl EngineState {
    pub fn name(self) -> &'static str {
        match self {
            EngineState::Running => "Running",
            EngineState::WaitingForDevice => "WaitingForDevice",
            EngineState::Failed(_) => "Failed",
            EngineState::StoppedByUser => "StoppedByUser",
        }
    }

    /// 실패면 짧은 원인, 아니면 빈 문자열.
    pub fn cause(self) -> &'static str {
        match self {
            EngineState::Failed(cause) => cause.as_str(),
            _ => "",
        }
    }

    fn code(self) -> u8 {
        match self {
            EngineState::Running => 0,
            EngineState::WaitingForDevice => 1,
            EngineState::Failed(FailCause::StartHung) => 2,
            EngineState::Failed(FailCause::OpenFailed) => 3,
            EngineState::Failed(FailCause::Unavailable) => 4,
            EngineState::StoppedByUser => 5,
        }
    }

    fn from_code(code: u8) -> Self {
        match code {
            0 => EngineState::Running,
            1 => EngineState::WaitingForDevice,
            2 => EngineState::Failed(FailCause::StartHung),
            3 => EngineState::Failed(FailCause::OpenFailed),
            4 => EngineState::Failed(FailCause::Unavailable),
            _ => EngineState::StoppedByUser,
        }
    }
}

/// 엔진을 열지 못한 이유.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartFailure {
    /// 장치를 찾지 못했다(아직 안 켜짐·뽑힘). 기다리면 나타날 수 있다.
    NotFound(String),
    /// 장치는 찾았는데 열지 못했다(스트림 만들기·시작 실패, 다른 프로그램이 점유 등).
    OpenFailed(String),
    /// 시작 호출이 시간 안에 돌아오지 않았다.
    Hung,
    /// 이 PC에서 나타날 수 없는 장치다.
    Unavailable(String),
}

impl StartFailure {
    pub fn message(&self) -> String {
        match self {
            StartFailure::NotFound(m) | StartFailure::OpenFailed(m) | StartFailure::Unavailable(m) => m.clone(),
            StartFailure::Hung => "오디오 드라이버가 응답하지 않습니다. 앱을 다시 시작해야 합니다".to_string(),
        }
    }
}

/// 다시 찾는 간격(초 단위, 사용자 결정 U1). 그 뒤는 마지막 값을 되풀이한다.
pub const RETRY_DELAYS_S: [u64; 6] = [1, 2, 4, 8, 16, 30];
/// 장치는 찾았는데 열기가 이만큼 연속 실패하면 실패 상태로 본다(사용자 결정 U3).
pub const OPEN_FAIL_LIMIT: u32 = 3;
/// 대기·실패 상태에서 진행 카운터를 올리는 간격(감시가 "유지자가 살아 있다"를 본다).
pub const PROGRESS_TICK_S: u64 = 5;
/// 유지자가 장치를 기다리며 다시 시도할 때 찾는 시간(설계 D4). 첫 시작·자기 재시작은 엔진 기본 30초.
pub const RETRY_SEARCH: Duration = Duration::from_secs(5);

/// `attempt`번째(1부터) 실패 뒤 다음 시도까지의 간격(단위 수).
pub fn retry_delay_units(attempt: u32) -> u64 {
    let i = (attempt.max(1) as usize - 1).min(RETRY_DELAYS_S.len() - 1);
    RETRY_DELAYS_S[i]
}

/// 순수 상태 기계(시험 대상). 시간·스레드·장치는 모른다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeeperCore {
    pub state: EngineState,
    /// 이번 끊김에서 실패한 시도 수.
    pub attempts: u32,
    /// "장치는 찾았는데 열기 실패"가 연속된 수.
    pub open_failures: u32,
}

impl Default for KeeperCore {
    fn default() -> Self {
        Self { state: EngineState::StoppedByUser, attempts: 0, open_failures: 0 }
    }
}

impl KeeperCore {
    pub fn on_started(&mut self) {
        self.state = EngineState::Running;
        self.attempts = 0;
        self.open_failures = 0;
    }

    /// 실패를 반영한다. 다시 시도하면 다음 간격(단위 수)을, 더 시도하지 않으면 None을 돌려준다.
    pub fn on_failed(&mut self, failure: &StartFailure) -> Option<u64> {
        self.attempts = self.attempts.saturating_add(1);
        match failure {
            StartFailure::Hung => {
                self.state = EngineState::Failed(FailCause::StartHung);
                None
            }
            StartFailure::Unavailable(_) => {
                self.state = EngineState::Failed(FailCause::Unavailable);
                None
            }
            StartFailure::OpenFailed(_) => {
                self.open_failures = self.open_failures.saturating_add(1);
                self.state = if self.open_failures >= OPEN_FAIL_LIMIT {
                    EngineState::Failed(FailCause::OpenFailed)
                } else {
                    EngineState::WaitingForDevice
                };
                Some(retry_delay_units(self.attempts))
            }
            StartFailure::NotFound(_) => {
                self.open_failures = 0;
                self.state = EngineState::WaitingForDevice;
                Some(retry_delay_units(self.attempts))
            }
        }
    }

    pub fn on_user_stop(&mut self) {
        *self = KeeperCore::default();
    }
}

/// 원하는 엔진 상태(사용자가 정한다).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Desired {
    Run(Option<String>),
    Stop,
}

/// 엔진 열기 요청.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenRequest {
    /// 열 장치(None = 기본 장치).
    pub device: Option<String>,
    /// 자기 재시작(엔진이 스스로 내려간 직후)인가. 아니면 장치를 기다리며 하는 재시도다.
    pub self_restart: bool,
}

/// 엔진을 여는 함수. 기본은 실제 엔진(`api::simple::keeper_open`), 시험은 흉내 함수를 넣는다.
pub type OpenFn = fn(&OpenRequest) -> Result<(), StartFailure>;
/// 다시 연 뒤 재개하는 함수. 기본은 `restart_resume::resume_after_resync`.
pub type ResumeFn = fn();

struct Runtime {
    desired: Desired,
    core: KeeperCore,
    next_try: Option<Instant>,
    /// 엔진 스레드가 알린 자기 재시작(세대, 장치).
    self_restart: Option<(u64, Option<String>)>,
    thread_started: bool,
    last_retry_log: Option<Instant>,
}

static RUNTIME: Mutex<Runtime> = Mutex::new(Runtime {
    desired: Desired::Stop,
    core: KeeperCore { state: EngineState::StoppedByUser, attempts: 0, open_failures: 0 },
    next_try: None,
    self_restart: None,
    thread_started: false,
    last_retry_log: None,
});
static WAKE: Condvar = Condvar::new();
/// 엔진 열기를 한 번에 하나만(사용자 시작·장치 변경과 유지자 재시도가 겹치지 않게).
static OPEN_LOCK: Mutex<()> = Mutex::new(());
/// 진행 카운터(하트비트 `engine_progress`).
static PROGRESS: AtomicU64 = AtomicU64::new(0);
/// 상태 사본(잠금 없이 읽는다).
static STATE_CODE: AtomicU8 = AtomicU8::new(5);
/// 간격 단위(밀리초). 실제 1000, 시험은 줄인다.
static UNIT_MS: AtomicU64 = AtomicU64::new(1000);
static HOOKS: RwLock<(Option<OpenFn>, Option<ResumeFn>)> = RwLock::new((None, None));

fn runtime() -> MutexGuard<'static, Runtime> {
    RUNTIME.lock().unwrap_or_else(|e| e.into_inner())
}

fn unit(units: u64) -> Duration {
    Duration::from_millis(units.saturating_mul(UNIT_MS.load(Ordering::Relaxed)))
}

/// 시험 전용: 열기·재개 함수와 간격 단위(밀리초)를 바꾼다.
pub fn set_test_hooks(opener: OpenFn, resume: ResumeFn, unit_ms: u64) {
    *HOOKS.write().unwrap_or_else(|e| e.into_inner()) = (Some(opener), Some(resume));
    UNIT_MS.store(unit_ms.max(1), Ordering::Relaxed);
}

fn opener() -> OpenFn {
    HOOKS.read().unwrap_or_else(|e| e.into_inner()).0.unwrap_or(crate::api::simple::keeper_open)
}

fn resumer() -> ResumeFn {
    HOOKS.read().unwrap_or_else(|e| e.into_inner()).1.unwrap_or(crate::core::restart_resume::resume_after_resync)
}

/// 지금 상태.
pub fn state() -> EngineState {
    EngineState::from_code(STATE_CODE.load(Ordering::Acquire))
}

/// 진행 카운터.
pub fn progress() -> u64 {
    PROGRESS.load(Ordering::Acquire)
}

/// 재생 명령을 받는가. 장치를 기다리거나 실패한 동안의 재생 명령은 쌓지 않고 버린다(설계 D11, OSC 규격 §4).
pub fn accepts_play() -> bool {
    !matches!(state(), EngineState::WaitingForDevice | EngineState::Failed(_))
}

/// 시작 호출이 멈춘 뒤인가. 멈춘 엔진 스레드를 다시 join하면 같이 멈추므로 새 시작을 바로 거절한다.
pub fn is_hung() -> bool {
    state() == EngineState::Failed(FailCause::StartHung)
}

/// 엔진 열기 잠금. 사용자 시작·장치 변경(`api_init_audio_system`)과 유지자가 함께 쓴다.
pub fn open_lock() -> MutexGuard<'static, ()> {
    OPEN_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// 상태 방송 문자열. 대기는 `WaitingForDevice:<시도 수>:<다음 시도까지 초>`, 실패는 `EngineDown:<원인>`(지속 배너).
pub fn status_string(core: &KeeperCore, next_in: Option<Duration>) -> Option<String> {
    match core.state {
        EngineState::WaitingForDevice => Some(format!(
            "WaitingForDevice:{}:{}",
            core.attempts,
            next_in.map(|d| d.as_secs()).unwrap_or(0)
        )),
        EngineState::Failed(cause) => Some(crate::core::event_log::engine_down_status(cause.as_str())),
        _ => None,
    }
}

/// 상태를 바꾸고 사본·카운터·방송·사건 로그를 맞춘다. 잠금을 쥔 채 부른다.
fn publish(rt: &mut Runtime, before: EngineState, failure: Option<&StartFailure>) {
    let after = rt.core.state;
    STATE_CODE.store(after.code(), Ordering::Release);
    if before != after {
        crate::core::event_log::log_event(
            "engine_state",
            &[
                ("from", before.name()),
                ("to", after.name()),
                ("cause", after.cause()),
                ("attempt", &rt.core.attempts.to_string()),
            ],
        );
        if let EngineState::Failed(cause) = after {
            GLOBAL_STATE.log(format!(
                "엔진 유지: 실패 상태({}) — {}",
                cause.as_str(),
                failure.map(StartFailure::message).unwrap_or_default()
            ));
        }
    }
    let next_in = rt.next_try.map(|t| t.saturating_duration_since(Instant::now()));
    if let Some(status) = status_string(&rt.core, next_in) {
        crate::api::simple::broadcast_stream_status(status);
    }
    if after == EngineState::WaitingForDevice {
        let quiet = rt.last_retry_log.is_some_and(|t| t.elapsed() < Duration::from_secs(60));
        if !quiet {
            rt.last_retry_log = Some(Instant::now());
            GLOBAL_STATE.log(format!(
                "엔진 유지: 오디오 장치를 기다리는 중 — {}번 시도, 다음 시도 {}초 뒤({})",
                rt.core.attempts,
                next_in.map(|d| d.as_secs()).unwrap_or(0),
                failure.map(StartFailure::message).unwrap_or_default()
            ));
        }
    }
}

/// 실패를 반영하고 다음 시도를 정한다. 잠금을 쥔 채 부른다.
fn apply_failure(rt: &mut Runtime, failure: &StartFailure) {
    let before = rt.core.state;
    rt.next_try = rt.core.on_failed(failure).map(|units| Instant::now() + unit(units));
    PROGRESS.fetch_add(1, Ordering::AcqRel);
    publish(rt, before, Some(failure));
}

fn apply_success(rt: &mut Runtime) {
    let before = rt.core.state;
    rt.core.on_started();
    rt.next_try = None;
    rt.last_retry_log = None;
    PROGRESS.fetch_add(1, Ordering::AcqRel);
    publish(rt, before, None);
}

fn ensure_thread(rt: &mut Runtime) {
    if rt.thread_started {
        return;
    }
    let spawned = std::thread::Builder::new().name("engine-keeper".into()).spawn(keeper_loop);
    match spawned {
        Ok(_) => rt.thread_started = true,
        Err(e) => GLOBAL_STATE.log(format!("엔진 유지 스레드를 시작하지 못했다: {e}")),
    }
}

/// 사용자가 엔진을 시작하거나 장치를 바꾼 결과(`api_init_audio_system`이 열기 잠금을 쥔 채 부른다).
/// 실패해도 장치가 나타날 수 있으면 유지자가 계속 찾는다.
pub fn user_started(device: Option<String>, result: &Result<(), StartFailure>) {
    let mut rt = runtime();
    rt.desired = Desired::Run(device);
    rt.self_restart = None;
    rt.core.attempts = 0;
    rt.core.open_failures = 0;
    match result {
        Ok(()) => apply_success(&mut rt),
        Err(failure) => apply_failure(&mut rt, failure),
    }
    ensure_thread(&mut rt);
    WAKE.notify_all();
}

/// 사용자가 엔진을 정지했다. 재시도·자기 재시작 대기를 버린다.
pub fn user_stopped() {
    let mut rt = runtime();
    let before = rt.core.state;
    rt.desired = Desired::Stop;
    rt.self_restart = None;
    rt.next_try = None;
    rt.core.on_user_stop();
    publish(&mut rt, before, None);
    WAKE.notify_all();
    drop(rt);
    // 화면의 대기·실패 배너를 내린다(사용자가 일부러 정지했다).
    crate::api::simple::broadcast_stream_status(EngineState::StoppedByUser.name().to_string());
}

/// 지금 상태의 방송 문자열(대기·실패일 때만). 상태 스트림을 새로 구독하면 처음 값으로 보낸다.
pub fn current_status_string() -> Option<String> {
    let rt = runtime();
    let next_in = rt.next_try.map(|t| t.saturating_duration_since(Instant::now()));
    status_string(&rt.core, next_in)
}

/// 엔진 스레드가 점검 루프를 빠져나와 자기 재시작이 필요할 때 부른다(`generation`은 그 엔진의 세대).
pub fn report_self_restart(generation: u64, device: Option<String>) {
    let mut rt = runtime();
    rt.self_restart = Some((generation, device));
    ensure_thread(&mut rt);
    WAKE.notify_all();
}

enum Job {
    SelfRestart(u64, Option<String>),
    Retry(Option<String>),
}

/// 다음 일을 기다린다. 대기·실패 상태에서는 진행 카운터를 PROGRESS_TICK마다 올린다.
fn next_job() -> Job {
    let mut rt = runtime();
    loop {
        if let Some((generation, device)) = rt.self_restart.take() {
            return Job::SelfRestart(generation, device);
        }
        let now = Instant::now();
        if let (Desired::Run(device), Some(at)) = (rt.desired.clone(), rt.next_try) {
            if now >= at {
                rt.next_try = None;
                return Job::Retry(device);
            }
        }
        let tick = unit(PROGRESS_TICK_S);
        let wait = rt.next_try.map(|at| at.saturating_duration_since(now).min(tick)).unwrap_or(tick);
        let (guard, timeout) = WAKE.wait_timeout(rt, wait).unwrap_or_else(|e| e.into_inner());
        rt = guard;
        if timeout.timed_out() && matches!(rt.core.state, EngineState::WaitingForDevice | EngineState::Failed(_)) {
            PROGRESS.fetch_add(1, Ordering::AcqRel);
        }
    }
}

/// 열기 결과를 반영한다(열기 잠금 안에서). 다시 연 엔진을 재개해야 하면 true.
/// 여는 사이 사용자가 정지했으면 연 엔진을 곧바로 내리고 정지 상태를 지킨다.
fn settle(result: Result<(), StartFailure>) -> bool {
    let mut rt = runtime();
    if rt.desired == Desired::Stop {
        drop(rt);
        if result.is_ok() {
            crate::api::simple::keeper_stop_engine();
        }
        return false;
    }
    match &result {
        Ok(()) => apply_success(&mut rt),
        Err(failure) => apply_failure(&mut rt, failure),
    }
    result.is_ok()
}

fn keeper_loop() {
    loop {
        match next_job() {
            Job::SelfRestart(generation, device) => {
                // 옛 엔진이 내려간 뒤 잠깐 둔다(예전 재연결 스레드와 같은 100ms).
                std::thread::sleep(Duration::from_millis(100));
                let started = {
                    let _open = open_lock();
                    if crate::audio::engine::ENGINE_GENERATION.load(Ordering::SeqCst) != generation {
                        continue; // 그 사이 사용자가 직접 정지·재기동했다
                    }
                    let result = opener()(&OpenRequest { device, self_restart: true });
                    settle(result)
                };
                if started {
                    resumer()();
                }
            }
            Job::Retry(device) => {
                let started = {
                    let _open = open_lock();
                    {
                        let rt = runtime();
                        // 잠금을 기다리는 사이 사용자가 정지하거나 장치를 바꿨으면 이 시도는 버린다.
                        if rt.desired != Desired::Run(device.clone()) || rt.core.state == EngineState::Running {
                            continue;
                        }
                    }
                    let result = opener()(&OpenRequest { device, self_restart: false });
                    settle(result)
                };
                if started {
                    resumer()();
                }
            }
        }
    }
}
