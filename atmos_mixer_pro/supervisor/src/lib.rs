//! 충돌·멈춤 뒤 Atmos Mixer Pro를 다시 띄우는 감시 프로그램
//! (docs/superpowers/specs/2026-10-05-crash-relaunch-supervisor-design.md).
//! 판단 규칙은 프로세스 없이 테스트할 수 있게 모듈로 나눈다.
pub mod backoff;
pub mod heartbeat;
pub mod logfile;
pub mod process;
pub mod run;
pub mod watch;
