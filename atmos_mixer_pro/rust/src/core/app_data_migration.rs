//! 앱 데이터 폴더 옮기기(사용자 결정 2026-11-22 "10번은 지금 프로그램 이름으로 바꿔", 2026-11-16 "com.example: 나 (나라면
//! 쓸 이름:the awesome)").
//!
//! Windows 앱 데이터 폴더는 실행 파일의 회사·제품 이름으로 정해진다(path_provider_windows: `%APPDATA%\<CompanyName>\
//! <ProductName>`). 회사 이름을 Flutter 기본값 `com.example`에서 `The Awesome`(windows/runner/Runner.rc)으로 바꿔 폴더가
//! `%APPDATA%\The Awesome\atmos_mixer_pro`가 된다. 새 버전이 처음 뜰 때, 설정·설계 데이터·공연 상태를 읽기 전에 옛 폴더를
//! 새 폴더로 **복사**한다. 옮기지 않으므로 옛 버전으로 되돌려 설치해도 옛 데이터가 남는다(그 뒤 새 버전에서 바꾼 것은
//! 옛 폴더에 없다). 감시 프로그램은 새 버전이 아직 한 번도 뜨지 않아 새 폴더가 없으면 옛 폴더의 "자동 실행 끔" 표시를
//! 본다(supervisor/src/run.rs). macOS는 번들 ID를 바꾸지 않았다(환경설정 저장소까지 옮겨야 해 Mac 쪽 몫).
use std::path::{Path, PathBuf};

/// 옛 폴더를 복사했다는 표시 파일(새 폴더 안). 내용은 복사한 시각과 옛 폴더 경로.
pub const MARKER: &str = "migrated_from_com_example";

/// 폴더에 앱 데이터가 있다고 보는 파일: 설정과 설계 데이터. 새 폴더에 둘 중 하나라도 있으면 복사하지 않는다(이미
/// 옮겼거나 새 버전에서 만든 데이터를 덮지 않는다).
const DATA_FILES: [&str; 2] = ["config.json", "shared_preferences.json"];

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// 새 폴더에 이미 데이터가 있거나 옛 폴더에 데이터가 없다.
    NotNeeded,
    /// 파일 n개를 복사했다.
    Copied(usize),
    /// 복사하다 실패했다. 복사한 파일은 지워 다음 시작 때 처음부터 다시 한다.
    Failed(String),
}

/// 옛 앱 데이터 폴더(`<appdata>\com.example\atmos_mixer_pro`). 새 폴더와 같으면 None(회사 이름을 바꾸지 않은 빌드).
pub fn legacy_dir(appdata: &Path, new_dir: &Path) -> Option<PathBuf> {
    let legacy = appdata.join("com.example").join("atmos_mixer_pro");
    (legacy != new_dir).then_some(legacy)
}

fn has_data(dir: &Path) -> bool {
    DATA_FILES.iter().any(|name| dir.join(name).is_file())
}

/// 옛 폴더를 새 폴더로 복사한다. 새 폴더에 이미 있는 파일은 건드리지 않는다.
pub fn migrate(legacy: &Path, new_dir: &Path) -> Outcome {
    if has_data(new_dir) || !has_data(legacy) {
        return Outcome::NotNeeded;
    }
    let mut copied = Vec::new();
    match copy_tree(legacy, new_dir, &mut copied) {
        Ok(()) => {
            let note = format!("{}\n{}\n", crate::core::app_signals::now_ms(), legacy.display());
            // 표시 파일은 기록일 뿐이다 — 데이터 파일이 새 폴더에 있으므로 쓰지 못해도 다시 복사하지 않는다.
            let _ = std::fs::write(new_dir.join(MARKER), note);
            Outcome::Copied(copied.len())
        }
        Err(e) => {
            for path in copied.iter().rev() {
                let _ = std::fs::remove_file(path);
            }
            Outcome::Failed(e.to_string())
        }
    }
}

fn copy_tree(from: &Path, to: &Path, copied: &mut Vec<PathBuf>) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target, copied)?;
        } else if !target.exists() {
            std::fs::copy(entry.path(), &target)?;
            copied.push(target);
        }
    }
    Ok(())
}

/// 앱 시작 때 공연 상태·설정·설계 데이터를 읽기 전에 한 번 부른다(api::show::api_start_show_state). Windows에서만 옛
/// 폴더를 본다. 복사했거나 실패했으면 앱 로그에 한 줄 남긴다.
pub fn run_at_startup(new_dir: &Path) {
    if !cfg!(windows) {
        return;
    }
    let Some(appdata) = std::env::var_os("APPDATA") else {
        return;
    };
    let Some(legacy) = legacy_dir(Path::new(&appdata), new_dir) else {
        return;
    };
    let state = &crate::core::state::GLOBAL_STATE;
    match migrate(&legacy, new_dir) {
        Outcome::NotNeeded => {}
        Outcome::Copied(count) => state.log(format!(
            "앱 데이터 폴더 옮김: {} → {} (파일 {count}개 복사, 옛 폴더는 그대로 둠)",
            legacy.display(),
            new_dir.display()
        )),
        Outcome::Failed(error) => state.log(format!(
            "앱 데이터 폴더 옮기기 실패: {} → {} ({error}) — 복사한 파일을 지웠다, 다음 시작 때 다시 한다",
            legacy.display(),
            new_dir.display()
        )),
    }
}
