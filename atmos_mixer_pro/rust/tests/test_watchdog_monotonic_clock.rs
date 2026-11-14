//! 엔진 워치독(콜백이 1초 넘게 멈추면 엔진 재기동)은 단조 시계로 잰다.
//!
//! 예전에는 콜백이 벽시계(`SystemTime`)로 시각을 남기고 점검 루프도 벽시계와 비교했다. Windows 시간 동기화가
//! 시계를 1초 넘게 앞으로 옮기면 콜백이 멀쩡히 돌아도 공백으로 오판해 엔진을 재기동할 수 있었다(약 1초 끊김,
//! 검토 보고서 A2). 감시 프로그램은 같은 이유로 이미 단조 시계를 쓴다(supervisor/src/watch.rs).
//! 시계 조정을 시험에서 일으킬 수 없으므로 소스 감사로 고정하고, 단조 시각 함수의 성질을 확인한다.
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;

fn source(path: &str) -> String {
    std::fs::read_to_string(format!("{}/{}", env!("CARGO_MANIFEST_DIR"), path))
        .unwrap_or_else(|e| panic!("{path}를 읽을 수 없다: {e}"))
}

/// `src`에서 `start` 표식부터 `end` 표식까지(끝 표식 포함 전까지).
fn region<'a>(src: &'a str, start: &str, end: &str) -> &'a str {
    let from = src.find(start).unwrap_or_else(|| panic!("표식을 찾을 수 없다: {start}"));
    let len = src[from..].find(end).unwrap_or_else(|| panic!("끝 표식을 찾을 수 없다: {end}"));
    &src[from..from + len]
}

#[test]
fn 콜백은_워치독_시각을_단조_시계로_남긴다() {
    let engine = source("src/audio/engine.rs");
    let body = region(&engine, "pub fn begin_callback(", "Self::process_commands(mixer, rx);");
    assert!(body.contains("watchdog_last_callback"), "begin_callback이 워치독 시각을 남기지 않는다");
    assert!(!body.contains("SystemTime"), "콜백이 워치독 시각을 벽시계로 남긴다: 시계 조정에 오발한다");
    assert!(body.contains("monotonic_ms()"), "콜백이 단조 시각(GLOBAL_STATE.monotonic_ms)을 쓰지 않는다");
}

#[test]
fn 점검_루프는_워치독_공백을_단조_시계로_잰다() {
    let simple = source("src/api/simple.rs");
    let block = region(&simple, "// Watchdog: Check if callback", "stop_reason = \"callback_gap_watchdog\";");
    assert!(!block.contains("SystemTime"), "점검 루프가 벽시계로 공백을 잰다: 시계 조정에 오발한다");
    assert!(block.contains("monotonic_ms()"), "점검 루프가 단조 시각(GLOBAL_STATE.monotonic_ms)을 쓰지 않는다");
}

#[test]
fn 단조_시각은_1부터_시작하고_줄지_않는다() {
    // 0은 "아직 콜백 없음"(엔진 종료 때 0으로 되돌림, 감시는 0을 진행으로 보지 않음)이라 단조 시각은 1 이상이다.
    let a = GLOBAL_STATE.monotonic_ms();
    std::thread::sleep(std::time::Duration::from_millis(20));
    let b = GLOBAL_STATE.monotonic_ms();
    assert!(a >= 1, "단조 시각이 0이다");
    assert!(b >= a + 15, "20ms 뒤 단조 시각이 충분히 늘지 않았다: {a} → {b}");
    // 벽시계(1970년 기준, 1.7조 ms 넘음)가 아니라 앱 시작 기준 경과 시간이다.
    assert!(b < 1_000_000_000, "단조 시각이 벽시계 값처럼 크다: {b}");
}
