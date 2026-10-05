//! 감시 기록(supervisor.log). 앱 로그(rust/src/core/log_file.rs)와 같은 규칙으로 민다: 10MB를 넘으면
//! .1, .2 …로 밀고 4개까지 남긴다. 감시는 앱 라이브러리를 쓰지 않으므로 규칙을 여기에 둔다.
use std::io::Write;
use std::path::Path;

pub const SUPERVISOR_LOG: &str = "supervisor.log";
pub const MAX_LOG_BYTES: u64 = 10 * 1024 * 1024;
pub const KEEP_ROTATED: usize = 4;

/// [dir]/[name]에 한 줄을 덧붙인다. 지금 파일이 [max_bytes] 이상이면 먼저 민다. 밀기에 실패해도
/// (Windows에서 다른 프로그램이 기록 파일을 쥐고 있으면 이름 바꾸기가 실패한다) 그 줄은 지금 파일에 남긴다.
pub fn append_line(dir: &Path, name: &str, line: &str, max_bytes: u64, keep: usize) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let current = dir.join(name);
    if std::fs::metadata(&current).map(|m| m.len() >= max_bytes).unwrap_or(false) {
        let _ = rotate(dir, name, keep);
    }
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&current)?;
    writeln!(file, "{line}")
}

/// 지금 파일을 옆 이름으로 먼저 옮겨 본다. 그게 안 되면 아무것도 건드리지 않는다 — 오래된 파일부터 밀면
/// 지금 파일에서 실패할 때마다(줄마다) 다시 밀면서 오래된 기록을 하나씩 지운다.
fn rotate(dir: &Path, name: &str, keep: usize) -> std::io::Result<()> {
    let current = dir.join(name);
    if keep == 0 {
        return std::fs::remove_file(current);
    }
    let rotating = dir.join(format!("{name}.rotating"));
    std::fs::rename(&current, &rotating)?;
    let rotated = |n: usize| dir.join(format!("{name}.{n}"));
    let _ = std::fs::remove_file(rotated(keep));
    for n in (1..keep).rev() {
        let from = rotated(n);
        if from.exists() {
            let _ = std::fs::rename(&from, rotated(n + 1));
        }
    }
    std::fs::rename(rotating, rotated(1))
}

/// 감시 기록 한 줄(현지 시각 + 내용). 기록에 실패해도 감시는 계속한다.
pub fn log(dir: &Path, msg: &str) {
    let time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let _ = append_line(dir, SUPERVISOR_LOG, &format!("[{time}] {msg}"), MAX_LOG_BYTES, KEEP_ROTATED);
}
