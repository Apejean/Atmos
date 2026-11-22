//! 앱 데이터 폴더 옮기기(`%APPDATA%\com.example\atmos_mixer_pro` → `%APPDATA%\The Awesome\atmos_mixer_pro`, 사용자 결정
//! 2026-11-22 "10번은 지금 프로그램 이름으로 바꿔"). 새 버전이 처음 뜰 때 옛 폴더를 복사하고(옛 폴더는 그대로), 이미
//! 옮겼거나 새 데이터가 있으면 건드리지 않는다.
use rust_lib_atmos_mixer_pro::core::app_data_migration::{legacy_dir, migrate, Outcome, MARKER};
use std::path::{Path, PathBuf};

fn fresh_appdata(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("atmos_app_data_migration_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp appdata");
    dir
}

fn legacy(appdata: &Path) -> PathBuf {
    appdata.join("com.example").join("atmos_mixer_pro")
}

fn current(appdata: &Path) -> PathBuf {
    appdata.join("The Awesome").join("atmos_mixer_pro")
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, text).expect("write");
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

/// 이 PC의 현장 데이터와 같은 파일들(설정·직전 정상본·설계 데이터·공연 상태·자동 실행 끔 표시).
fn fill_legacy(appdata: &Path) {
    let old = legacy(appdata);
    write(&old.join("config.json"), "{\"osc_port\":8000}");
    write(&old.join("config.json.bak"), "{\"osc_port\":7000}");
    write(&old.join("shared_preferences.json"), "{\"flutter.k\":1}");
    write(&old.join("show_state.json"), "{\"tracks\":[]}");
    write(&old.join("windows_autostart_off"), "");
}

#[test]
fn 옛_폴더만_있으면_새_폴더로_복사하고_옛_폴더는_남긴다() {
    let appdata = fresh_appdata("copy");
    fill_legacy(&appdata);
    // path_provider_windows는 폴더를 돌려주기 전에 만든다 — 빈 새 폴더가 있는 상태에서 옮긴다.
    std::fs::create_dir_all(current(&appdata)).expect("new dir");

    let outcome = migrate(&legacy(&appdata), &current(&appdata));

    assert_eq!(outcome, Outcome::Copied(5));
    for name in ["config.json", "config.json.bak", "shared_preferences.json", "show_state.json", "windows_autostart_off"] {
        assert!(current(&appdata).join(name).is_file(), "{name}을 복사하지 않았다");
        assert!(legacy(&appdata).join(name).is_file(), "옛 폴더의 {name}이 없어졌다");
    }
    assert_eq!(read(&current(&appdata).join("config.json")), "{\"osc_port\":8000}");
    assert!(read(&current(&appdata).join(MARKER)).contains("com.example"), "표시 파일에 옛 폴더 경로가 없다");
    let _ = std::fs::remove_dir_all(appdata);
}

#[test]
fn 새_폴더에_데이터가_있으면_옛_폴더를_복사하지_않는다() {
    let appdata = fresh_appdata("keep_new");
    fill_legacy(&appdata);
    write(&current(&appdata).join("config.json"), "{\"osc_port\":9000}");

    assert_eq!(migrate(&legacy(&appdata), &current(&appdata)), Outcome::NotNeeded);
    assert_eq!(read(&current(&appdata).join("config.json")), "{\"osc_port\":9000}", "새 설정을 덮었다");
    assert!(!current(&appdata).join("shared_preferences.json").exists(), "새 데이터가 있는데 옛 파일을 섞었다");

    // 설계 데이터만 있어도 새 데이터로 본다.
    let appdata2 = fresh_appdata("keep_new_prefs");
    fill_legacy(&appdata2);
    write(&current(&appdata2).join("shared_preferences.json"), "{}");
    assert_eq!(migrate(&legacy(&appdata2), &current(&appdata2)), Outcome::NotNeeded);
    assert!(!current(&appdata2).join("config.json").exists());
    let _ = std::fs::remove_dir_all(appdata);
    let _ = std::fs::remove_dir_all(appdata2);
}

#[test]
fn 옛_폴더가_없거나_비어_있으면_아무것도_하지_않는다() {
    let appdata = fresh_appdata("no_legacy");
    assert_eq!(migrate(&legacy(&appdata), &current(&appdata)), Outcome::NotNeeded);
    std::fs::create_dir_all(legacy(&appdata)).expect("empty legacy");
    write(&legacy(&appdata).join("windows_autostart_off"), "");
    assert_eq!(migrate(&legacy(&appdata), &current(&appdata)), Outcome::NotNeeded, "데이터 없는 옛 폴더를 옮겼다");
    assert!(!current(&appdata).join(MARKER).exists());
    let _ = std::fs::remove_dir_all(appdata);
}

#[test]
fn 두_번_불러도_한_번만_복사한다() {
    let appdata = fresh_appdata("twice");
    fill_legacy(&appdata);
    assert_eq!(migrate(&legacy(&appdata), &current(&appdata)), Outcome::Copied(5));
    // 새 버전에서 설정을 바꾼 뒤 다시 시작해도 옛 값으로 되돌리지 않는다.
    write(&current(&appdata).join("config.json"), "{\"osc_port\":9100}");
    assert_eq!(migrate(&legacy(&appdata), &current(&appdata)), Outcome::NotNeeded);
    assert_eq!(read(&current(&appdata).join("config.json")), "{\"osc_port\":9100}");
    let _ = std::fs::remove_dir_all(appdata);
}

#[test]
fn 복사하다_실패하면_복사한_파일을_지워_다음_시작_때_다시_한다() {
    let appdata = fresh_appdata("fail");
    fill_legacy(&appdata);
    write(&legacy(&appdata).join("sub").join("inner.txt"), "x");
    // 새 폴더에 같은 이름의 파일이 있어 하위 폴더를 만들 수 없게 한다.
    write(&current(&appdata).join("sub"), "not a folder");

    let outcome = migrate(&legacy(&appdata), &current(&appdata));

    assert!(matches!(outcome, Outcome::Failed(_)), "{outcome:?}");
    for name in ["config.json", "config.json.bak", "shared_preferences.json", "show_state.json", MARKER] {
        assert!(!current(&appdata).join(name).exists(), "실패했는데 {name}이 남았다");
    }
    // 막던 것이 없어지면 다음 시작 때 처음부터 옮긴다.
    std::fs::remove_file(current(&appdata).join("sub")).expect("unblock");
    assert_eq!(migrate(&legacy(&appdata), &current(&appdata)), Outcome::Copied(6));
    let _ = std::fs::remove_dir_all(appdata);
}

#[test]
fn 옛_폴더는_appdata_아래_com_example이고_같은_폴더면_옮기지_않는다() {
    let appdata = Path::new("C:\\Users\\u\\AppData\\Roaming");
    assert_eq!(legacy_dir(appdata, &current(appdata)), Some(legacy(appdata)));
    assert_eq!(legacy_dir(appdata, &legacy(appdata)), None, "회사 이름을 바꾸지 않은 빌드인데 옮기려 했다");
}
