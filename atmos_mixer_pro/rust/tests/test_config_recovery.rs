//! 설정 파일 손상 복구(common::config::AppConfig::{save_with_backup, load_with_recovery}).
//! 각 시험은 자기 임시 폴더만 쓴다.
use rust_lib_atmos_mixer_pro::common::config::{
    config_backup_path, AppConfig, RecoveryReason, RecoverySource,
};
use std::fs;
use std::path::PathBuf;

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("atmos_cfg_recovery_{}_{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn cfg(port: u16) -> AppConfig {
    AppConfig { osc_port: port, ..AppConfig::default() }
}

fn corrupted_copies(d: &PathBuf) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(d)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.file_name().unwrap().to_string_lossy().contains(".corrupted-"))
        .collect();
    v.sort();
    v
}

#[test]
fn healthy_file_loads_without_notice() {
    let d = dir("healthy");
    let path = d.join("config.json");
    cfg(9001).save_with_backup(&path).unwrap();
    let (c, rec) = AppConfig::load_with_recovery(&path);
    assert_eq!(c.osc_port, 9001);
    assert!(rec.is_none());
    assert!(!d.join("config.tmp").exists(), "임시 파일이 남으면 안 된다");
}

#[test]
fn first_run_creates_default_without_notice() {
    let d = dir("first_run");
    let path = d.join("config.json");
    let (c, rec) = AppConfig::load_with_recovery(&path);
    assert_eq!(c.osc_port, AppConfig::default().osc_port);
    assert!(rec.is_none());
    assert!(path.exists(), "첫 실행에는 기본 설정 파일을 만든다");
}

#[test]
fn backup_keeps_previous_good_version() {
    let d = dir("backup");
    let path = d.join("config.json");
    cfg(9001).save_with_backup(&path).unwrap(); // 처음: 백업할 원본 없음
    cfg(9002).save_with_backup(&path).unwrap(); // 9001이 .bak으로
    let bak: AppConfig =
        serde_json::from_str(&fs::read_to_string(config_backup_path(&path)).unwrap()).unwrap();
    assert_eq!(bak.osc_port, 9001);
}

#[test]
fn corrupted_file_recovers_from_backup_and_keeps_corrupted_copy() {
    let d = dir("corrupt_backup");
    let path = d.join("config.json");
    cfg(9001).save_with_backup(&path).unwrap();
    cfg(9002).save_with_backup(&path).unwrap();
    fs::write(&path, "{\"osc_port\": 90").unwrap(); // 쓰다 만 파일

    let (c, rec) = AppConfig::load_with_recovery(&path);
    let rec = rec.expect("복구 알림이 있어야 한다");
    assert_eq!(c.osc_port, 9001, "직전 정상본으로 되살린다");
    assert_eq!(rec.source, RecoverySource::Backup);
    assert!(matches!(rec.reason, RecoveryReason::Corrupted { kept: Some(_), .. }));
    assert!(rec.notice().contains("직전에 정상이던 설정"));

    // config.json은 되살린 내용으로 다시 쓰고, 손상본은 따로 남는다.
    let again: AppConfig = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(again.osc_port, 9001);
    let copies = corrupted_copies(&d);
    assert_eq!(copies.len(), 1);
    assert_eq!(fs::read_to_string(&copies[0]).unwrap(), "{\"osc_port\": 90");
}

#[test]
fn corrupted_without_backup_starts_default_and_does_not_overwrite_original() {
    let d = dir("corrupt_nobackup");
    let path = d.join("config.json");
    fs::write(&path, "not json").unwrap();

    let (c, rec) = AppConfig::load_with_recovery(&path);
    let rec = rec.expect("알림이 있어야 한다");
    assert_eq!(c.osc_port, AppConfig::default().osc_port);
    assert_eq!(rec.source, RecoverySource::Default);
    assert!(rec.notice().contains("기본 설정으로 시작"));
    assert_eq!(fs::read_to_string(&path).unwrap(), "not json", "원본을 기본값으로 덮지 않는다");
    assert_eq!(corrupted_copies(&d).len(), 1);
}

#[test]
fn missing_file_recovers_from_backup() {
    let d = dir("missing_backup");
    let path = d.join("config.json");
    cfg(9001).save_with_backup(&path).unwrap();
    cfg(9002).save_with_backup(&path).unwrap();
    fs::remove_file(&path).unwrap(); // 예전 저장 방식에서 "지우고 바꾸는" 사이에 꺼진 상황

    let (c, rec) = AppConfig::load_with_recovery(&path);
    let rec = rec.expect("알림이 있어야 한다");
    assert_eq!(c.osc_port, 9001);
    assert_eq!(rec.reason, RecoveryReason::Missing);
    assert_eq!(rec.source, RecoverySource::Backup);
    assert!(path.exists());
}

#[test]
fn complete_temp_file_is_preferred_over_backup() {
    let d = dir("temp_first");
    let path = d.join("config.json");
    cfg(9001).save_with_backup(&path).unwrap();
    cfg(9002).save_with_backup(&path).unwrap();
    fs::remove_file(&path).unwrap();
    // 이름 바꾸기 직전에 꺼져 다 쓴 임시 파일만 남은 상황
    fs::write(d.join("config.tmp"), serde_json::to_string(&cfg(9003)).unwrap()).unwrap();

    let (c, rec) = AppConfig::load_with_recovery(&path);
    assert_eq!(c.osc_port, 9003);
    assert_eq!(rec.unwrap().source, RecoverySource::Temp);
}

#[test]
fn partial_temp_file_is_skipped() {
    let d = dir("temp_partial");
    let path = d.join("config.json");
    cfg(9001).save_with_backup(&path).unwrap();
    cfg(9002).save_with_backup(&path).unwrap();
    fs::remove_file(&path).unwrap();
    fs::write(d.join("config.tmp"), "{\"osc_po").unwrap();

    let (c, rec) = AppConfig::load_with_recovery(&path);
    assert_eq!(c.osc_port, 9001);
    assert_eq!(rec.unwrap().source, RecoverySource::Backup);
}

#[test]
fn repeated_corruption_keeps_every_copy() {
    let d = dir("repeat");
    let path = d.join("config.json");
    fs::write(&path, "bad 1").unwrap();
    let _ = AppConfig::load_with_recovery(&path);
    fs::write(&path, "bad 2").unwrap();
    let _ = AppConfig::load_with_recovery(&path);
    let copies = corrupted_copies(&d);
    assert_eq!(copies.len(), 2, "같은 초에 두 번 손상돼도 앞의 손상본을 덮지 않는다");
}

#[test]
fn project_file_save_makes_no_backup() {
    let d = dir("project");
    let path = d.join("show.atmos");
    cfg(9001).save_to_file(&path).unwrap();
    cfg(9002).save_to_file(&path).unwrap();
    assert!(!config_backup_path(&path).exists(), "프로젝트 폴더에는 .bak을 만들지 않는다");
    assert!(!d.join("show.tmp").exists());
}

#[test]
fn stale_temp_file_loses_to_newer_backup() {
    let d = dir("temp_stale");
    let path = d.join("config.json");
    // 오래전 충돌에서 남은 임시 파일
    fs::write(d.join("config.tmp"), serde_json::to_string(&cfg(8000)).unwrap()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    cfg(9001).save_with_backup(&path).unwrap();
    cfg(9002).save_with_backup(&path).unwrap(); // 9001이 .bak(임시 파일보다 새것)
    fs::write(&path, "broken").unwrap();

    let (c, rec) = AppConfig::load_with_recovery(&path);
    assert_eq!(c.osc_port, 9001, "더 새 백업이 오래된 임시 파일을 이긴다");
    assert_eq!(rec.unwrap().source, RecoverySource::Backup);
}

#[test]
fn healthy_file_removes_leftover_temp() {
    let d = dir("leftover");
    let path = d.join("config.json");
    cfg(9001).save_with_backup(&path).unwrap();
    fs::write(d.join("config.tmp"), serde_json::to_string(&cfg(7000)).unwrap()).unwrap();
    let (c, rec) = AppConfig::load_with_recovery(&path);
    assert_eq!(c.osc_port, 9001);
    assert!(rec.is_none());
    assert!(!d.join("config.tmp").exists(), "정상이면 남은 임시 파일은 쓰지 않고 지운다");
}

#[test]
fn rename_failure_keeps_original_and_temp_and_returns_err() {
    let d = dir("rename_fail");
    // 바꿔칠 자리에 폴더가 있으면 이름 바꾸기가 실패한다(다른 프로그램이 잡고 있는 경우와 같은 결과).
    let path = d.join("config.json");
    fs::create_dir_all(path.join("keep")).unwrap();
    let result = cfg(9001).save_with_backup(&path);
    assert!(result.is_err(), "저장 실패를 화면이 알 수 있게 Err여야 한다");
    assert!(path.join("keep").exists(), "원본 자리는 그대로다");
    assert!(d.join("config.tmp").exists(), "임시 파일은 남긴다");
}

#[test]
fn corrupted_copies_are_capped_at_five() {
    let d = dir("cap");
    let path = d.join("config.json");
    for i in 0..8 {
        fs::write(&path, format!("bad {i}")).unwrap();
        let _ = AppConfig::load_with_recovery(&path);
    }
    let copies = corrupted_copies(&d);
    assert_eq!(copies.len(), rust_lib_atmos_mixer_pro::common::config::MAX_CORRUPTED_COPIES);
    let newest = copies.iter().map(|p| fs::read_to_string(p).unwrap()).collect::<Vec<_>>();
    assert!(newest.contains(&"bad 7".to_string()), "가장 최근 손상본은 남는다");
    assert!(!newest.contains(&"bad 0".to_string()), "가장 오래된 손상본은 지운다");
}
