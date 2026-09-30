//! DSP Law 1(힙 할당 금지) / Law 2(락·블로킹 금지) 정적 감사.
//!
//! `std::env::var`는 전역 락을 잡고 결과 String을 힙에 할당한다. 오디오
//! 콜백이나 커맨드 핸들러 안에서 부르면 두 법칙을 동시에 어긴다.
//!
//! 실제로 binaural.rs의 방위각 갱신 루프(채널마다 반복)와 mixer/engine의
//! 핫 경로에 이 호출이 들어 있었다. 스피커를 드래그하면 방위각이 매 콜백
//! 갱신되면서 채널 수만큼 락+할당이 일어나 "드드륵" 하는 소리가 났다.
//!
//! 디버그 플래그는 `core::state::debug_flags`가 OnceLock으로 한 번만 읽는다.

use std::fs;
use std::path::Path;

/// 오디오 스레드에서 실행되는 파일들.
const AUDIO_THREAD_FILES: &[&str] = &[
    "src/audio/binaural.rs",
    "src/audio/mixer.rs",
    "src/audio/engine.rs",
    "src/audio/dsp.rs",
    "src/audio/reverb.rs",
    "src/audio/crossover.rs",
];

#[test]
fn audio_thread_files_do_not_call_env_var() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut offenders = Vec::new();

    for rel in AUDIO_THREAD_FILES {
        let path = root.join(rel);
        let src = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue, // 파일이 없으면 건너뛴다(구조 변경 대비)
        };
        for (i, line) in src.lines().enumerate() {
            // 주석은 제외한다(왜 금지인지 설명하는 문장이 들어 있다).
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            if line.contains("env::var") {
                offenders.push(format!("{rel}:{}: {}", i + 1, line.trim()));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "오디오 스레드 코드에서 std::env::var를 부르고 있다(락 + 힙 할당).\n\
         core::state::debug_flags처럼 OnceLock으로 한 번만 읽어야 한다:\n{}",
        offenders.join("\n")
    );
}
