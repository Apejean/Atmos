//! 앱 로그 파일. 크기를 넘으면 뒤로 밀고(.1, .2 …) 정한 개수만 남긴다.
//! 로그는 임시 폴더에 쌓이는데 Windows의 %TEMP%는 저절로 비워지지 않아, 회전이 없으면 무인 운영에서
//! 끝없이 커졌다. 오디오 스레드에서는 부르지 않는다(파일 입출력·잠금).
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const LOG_FILE_NAME: &str = "atmos_mixer_pro.log";
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

/// [dir]의 로그 파일에 한 줄을 덧붙인다. 지금 파일이 [max_bytes] 이상이면 먼저 민다.
pub fn append_line(dir: &Path, line: &str, max_bytes: u64, keep: usize) -> std::io::Result<()> {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::fs::create_dir_all(dir)?;
    let current = dir.join(LOG_FILE_NAME);
    if std::fs::metadata(&current).map(|m| m.len() >= max_bytes).unwrap_or(false) {
        rotate(dir, keep)?;
    }
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&current)?;
    writeln!(file, "{line}")
}

/// 가장 오래된 파일을 지우고 나머지를 한 칸씩 민다(지금 파일 → .1).
fn rotate(dir: &Path, keep: usize) -> std::io::Result<()> {
    let current = dir.join(LOG_FILE_NAME);
    if keep == 0 {
        return std::fs::remove_file(current);
    }
    let _ = std::fs::remove_file(rotated(dir, keep));
    for n in (1..keep).rev() {
        let from = rotated(dir, n);
        if from.exists() {
            std::fs::rename(&from, rotated(dir, n + 1))?;
        }
    }
    std::fs::rename(current, rotated(dir, 1))
}

/// 로그 파일(지금 것과 밀린 것)을 [dest] 폴더로 복사하고 복사한 개수를 돌려준다.
pub fn export_to(dir: &Path, dest: &Path) -> std::io::Result<usize> {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut copied = 0;
    let files = std::iter::once(dir.join(LOG_FILE_NAME)).chain((1..=KEEP_ROTATED).map(|n| rotated(dir, n)));
    for path in files {
        if let (true, Some(name)) = (path.exists(), path.file_name()) {
            std::fs::copy(&path, dest.join(name))?;
            copied += 1;
        }
    }
    Ok(copied)
}
