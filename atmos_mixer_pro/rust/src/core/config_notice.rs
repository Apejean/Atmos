//! 설정 파일을 되살렸을 때 화면에 한 번 보일 안내(common::config::AppConfig::load_with_recovery).
//! 엔진 상태가 아니라 한 번 꺼내 쓰는 문구라 상태 스트림에 싣지 않는다.

use std::sync::Mutex;

static NOTICE: Mutex<Option<String>> = Mutex::new(None);

/// 되살림 결과를 사건 로그에 남기고 화면 안내를 맡겨 둔다.
pub fn record(path: &std::path::Path, recovery: &crate::common::config::ConfigRecovery) {
    let (reason, source) = recovery.log_fields();
    let kept = match &recovery.reason {
        crate::common::config::RecoveryReason::Corrupted { kept: Some(k), .. } => k.clone(),
        _ => "none".to_string(),
    };
    crate::core::event_log::log_event(
        "config_recovered",
        &[
            ("path", &path.to_string_lossy()),
            ("reason", &reason),
            ("from", &source),
            ("kept", &kept),
        ],
    );
    *NOTICE.lock().unwrap_or_else(|e| e.into_inner()) = Some(recovery.notice());
}

/// 맡겨 둔 안내를 꺼낸다(한 번만).
pub fn take() -> Option<String> {
    NOTICE.lock().unwrap_or_else(|e| e.into_inner()).take()
}
