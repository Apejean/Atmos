//! 앱 로그 파일. 크기를 넘으면 뒤로 밀고(.1, .2 …) 정한 개수만 남긴다.
//! 로그는 임시 폴더에 쌓이는데 Windows의 %TEMP%는 저절로 비워지지 않아, 회전이 없으면 무인 운영에서
//! 끝없이 커졌다. 오디오 스레드에서는 부르지 않는다(파일 입출력·잠금).
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const LOG_FILE_NAME: &str = "atmos_mixer_pro.log";
/// 감시 프로그램(supervisor/)이 같은 폴더에 남기는 기록. 내보낼 때 함께 복사한다.
pub const SUPERVISOR_LOG_FILE_NAME: &str = "supervisor.log";
/// 파일 하나의 최대 크기. 넘으면 다음 줄을 쓰기 전에 민다.
pub const MAX_LOG_BYTES: u64 = 10 * 1024 * 1024;
/// 밀린 파일을 몇 개까지 남기나. 지금 파일까지 최대 5개(약 50MB).
pub const KEEP_ROTATED: usize = 4;

/// 쓰기·회전·내보내기를 한 번에 하나만 한다(여러 스레드가 로그를 남긴다).
static LOCK: Mutex<()> = Mutex::new(());

/// 로그 폴더(임시 폴더 아래).
pub fn log_dir() -> PathBuf {
    std::env::temp_dir().join("atmos_mixer_pro_logs")
}

fn rotated(dir: &Path, n: usize) -> PathBuf {
    dir.join(format!("{LOG_FILE_NAME}.{n}"))
}

/// [dir]의 로그 파일에 한 줄을 덧붙인다. 지금 파일이 [max_bytes] 이상이면 먼저 민다. 밀기에 실패해도
/// (Windows에서 다른 프로그램이 로그를 쥐고 있으면 이름 바꾸기가 실패한다) 그 줄은 지금 파일에 남긴다.
pub fn append_line(dir: &Path, line: &str, max_bytes: u64, keep: usize) -> std::io::Result<()> {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::fs::create_dir_all(dir)?;
    let current = dir.join(LOG_FILE_NAME);
    if std::fs::metadata(&current).map(|m| m.len() >= max_bytes).unwrap_or(false) {
        let _ = rotate(dir, keep);
    }
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&current)?;
    writeln!(file, "{line}")
}

/// 가장 오래된 파일을 지우고 나머지를 한 칸씩 민다(지금 파일 → .1). 지금 파일을 옆 이름으로 먼저 옮겨 보고,
/// 그게 안 되면 아무것도 건드리지 않는다 — 예전에는 오래된 파일부터 밀다가 지금 파일에서 실패해 그 줄을 버렸고,
/// 다음 줄마다 다시 밀면서 오래된 로그를 하나씩 지웠다.
fn rotate(dir: &Path, keep: usize) -> std::io::Result<()> {
    let current = dir.join(LOG_FILE_NAME);
    if keep == 0 {
        return std::fs::remove_file(current);
    }
    let rotating = dir.join(format!("{LOG_FILE_NAME}.rotating"));
    std::fs::rename(&current, &rotating)?;
    let _ = std::fs::remove_file(rotated(dir, keep));
    for n in (1..keep).rev() {
        let from = rotated(dir, n);
        if from.exists() {
            let _ = std::fs::rename(&from, rotated(dir, n + 1));
        }
    }
    std::fs::rename(rotating, rotated(dir, 1))
}

/// 로그 파일(앱 로그와 감시 기록, 지금 것과 밀린 것)을 [dest] 폴더로 복사하고 복사한 개수를 돌려준다.
pub fn export_to(dir: &Path, dest: &Path) -> std::io::Result<usize> {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut copied = 0;
    for name in [LOG_FILE_NAME, SUPERVISOR_LOG_FILE_NAME] {
        let files = std::iter::once(dir.join(name))
            .chain((1..=KEEP_ROTATED).map(|n| dir.join(format!("{name}.{n}"))));
        for path in files {
            if let (true, Some(file_name)) = (path.exists(), path.file_name()) {
                std::fs::copy(&path, dest.join(file_name))?;
                copied += 1;
            }
        }
    }
    Ok(copied)
}
