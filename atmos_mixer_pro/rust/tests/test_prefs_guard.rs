//! 설계 데이터(shared_preferences.json) 보호(검토 보고서 A3, 안 B). 각 시험은 자기 임시 폴더만 쓴다.
use rust_lib_atmos_mixer_pro::core::prefs_guard::{backup_if_changed, check_prefs_file, PrefsOutcome};
use std::fs;
use std::path::PathBuf;

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("atmos_prefs_guard_{}_{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

const GOOD: &str = r#"{"flutter.tuning_state":"{\"ch\":1}","flutter.exhibition_speaker_layout":"[1,2]"}"#;
const OLDER: &str = r#"{"flutter.tuning_state":"{\"ch\":0}"}"#;

fn bak(p: &PathBuf) -> PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(".bak");
    PathBuf::from(s)
}

fn corrupted(d: &PathBuf) -> Vec<PathBuf> {
    fs::read_dir(d)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.file_name().unwrap().to_string_lossy().contains(".corrupted-"))
        .collect()
}

#[test]
fn missing_file_does_nothing() {
    // 첫 실행, 그리고 macOS(NSUserDefaults라 이 파일이 없다).
    let d = dir("missing");
    let p = d.join("shared_preferences.json");
    assert_eq!(check_prefs_file(&p), PrefsOutcome::Missing);
    assert!(!p.exists() && !bak(&p).exists(), "아무 파일도 만들지 않는다");
}

#[test]
fn healthy_file_makes_backup_only() {
    let d = dir("healthy");
    let p = d.join("shared_preferences.json");
    fs::write(&p, GOOD).unwrap();
    assert_eq!(check_prefs_file(&p), PrefsOutcome::Healthy);
    assert_eq!(fs::read_to_string(&p).unwrap(), GOOD, "원본은 그대로다");
    assert_eq!(fs::read_to_string(bak(&p)).unwrap(), GOOD);
    assert!(corrupted(&d).is_empty());
}

#[test]
fn empty_file_is_restored_from_backup() {
    let d = dir("empty");
    let p = d.join("shared_preferences.json");
    fs::write(bak(&p), OLDER).unwrap();
    fs::write(&p, "").unwrap(); // 쓰다가 꺼진 0바이트 파일
    let out = check_prefs_file(&p);
    assert!(matches!(out, PrefsOutcome::Recovered { .. }), "{out:?}");
    assert_eq!(fs::read_to_string(&p).unwrap(), OLDER);
    assert_eq!(corrupted(&d).len(), 1);
    assert!(out.notice().unwrap().contains("직전 정상본"));
}

#[test]
fn truncated_without_backup_resets_to_empty_object_and_keeps_copy() {
    let d = dir("truncated");
    let p = d.join("shared_preferences.json");
    fs::write(&p, r#"{"flutter.tuning_state":"{\"c"#).unwrap();
    let out = check_prefs_file(&p);
    assert!(matches!(out, PrefsOutcome::Reset { .. }), "{out:?}");
    assert_eq!(fs::read_to_string(&p).unwrap(), "{}", "플러그인이 예외를 던지지 않게 빈 객체로 바꾼다");
    let copies = corrupted(&d);
    assert_eq!(copies.len(), 1);
    assert_eq!(fs::read_to_string(&copies[0]).unwrap(), r#"{"flutter.tuning_state":"{\"c"#);
    assert!(out.notice().unwrap().contains(".atmos"));
}

#[test]
fn non_object_json_is_treated_as_broken() {
    let d = dir("array");
    let p = d.join("shared_preferences.json");
    fs::write(bak(&p), GOOD).unwrap();
    fs::write(&p, "[1,2,3]").unwrap();
    assert!(matches!(check_prefs_file(&p), PrefsOutcome::Recovered { .. }));
    assert_eq!(fs::read_to_string(&p).unwrap(), GOOD);
}

#[test]
fn unreadable_path_changes_nothing() {
    let d = dir("locked");
    let p = d.join("shared_preferences.json");
    fs::write(bak(&p), GOOD).unwrap();
    fs::create_dir_all(p.join("x")).unwrap(); // 폴더 자리 → NotFound가 아닌 읽기 오류
    assert!(matches!(check_prefs_file(&p), PrefsOutcome::Unreadable { .. }));
    assert!(p.join("x").exists(), "읽지 못하면 아무것도 바꾸지 않는다");
    assert!(corrupted(&d).is_empty());
}

#[test]
fn periodic_backup_skips_broken_and_fresh_files() {
    let d = dir("periodic");
    let p = d.join("shared_preferences.json");
    // 반쯤 쓴 파일은 정상본으로 삼지 않는다.
    fs::write(&p, "{\"flutter.a\":").unwrap();
    assert!(!backup_if_changed(&p, std::time::Duration::ZERO));
    assert!(!bak(&p).exists());
    // 방금 바뀐 파일은 쓰기 중일 수 있으니 기다린다(최소 경과 시간).
    fs::write(&p, GOOD).unwrap();
    assert!(!backup_if_changed(&p, std::time::Duration::from_secs(60)));
    // 충분히 지난 정상 파일은 정상본이 된다. 같은 내용이면 다시 쓰지 않는다.
    assert!(backup_if_changed(&p, std::time::Duration::ZERO));
    assert_eq!(fs::read_to_string(bak(&p)).unwrap(), GOOD);
    assert!(!backup_if_changed(&p, std::time::Duration::ZERO));
}

#[test]
fn reading_does_not_block_a_concurrent_writer() {
    // Rust가 파일을 연 채로 있어도 다른 쪽이 덮어쓸 수 있어야 한다(std는 읽기·쓰기·삭제 공유로 연다).
    let d = dir("share");
    let p = d.join("shared_preferences.json");
    fs::write(&p, GOOD).unwrap();
    let _held = fs::File::open(&p).unwrap();
    fs::write(&p, OLDER).expect("열려 있어도 덮어쓰기가 되어야 한다");
    assert_eq!(fs::read_to_string(&p).unwrap(), OLDER);
}

/// 실측용(기본 실행 안 함): `ATMOS_HOLD_FILE`을 Rust std로 연 채 15초 동안 들고 있는다. 그동안 Dart가 같은 파일에
/// writeAsStringSync로 쓰는지 따로 확인한다(A3 조건 3).
#[test]
#[ignore]
fn hold_file_open_for_dart_write_measurement() {
    let p = std::env::var("ATMOS_HOLD_FILE").expect("ATMOS_HOLD_FILE");
    let _held = fs::File::open(&p).unwrap();
    println!("holding {p}");
    std::thread::sleep(std::time::Duration::from_secs(15));
}
