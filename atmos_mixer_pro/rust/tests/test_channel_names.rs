//! 채널 이름 고르기: 엔진이 연 ASIO 드라이버가 준 이름은 그 드라이버에만 쓰고, 나머지는 "Channel N"이다.
//! 실제 드라이버에서 이름을 묻는 부분(Windows ASIO)은 diag_asio_channel_names가 실기에서 본다.

use rust_lib_atmos_mixer_pro::audio::channel_names::{names_or_fallback, strip_common_suffix};

fn owned(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| n.to_string()).collect()
}

#[test]
fn a_device_number_shared_by_every_channel_is_removed() {
    // RME ASIO MADIface USB가 실제로 준 이름의 모양(2026-10-11 실기).
    let mut names = owned(&["Analog 1 (1)", "AES 1 (1)", "", "MADI 64 (1)"]);
    assert_eq!(strip_common_suffix(&mut names).as_deref(), Some(" (1)"));
    assert_eq!(names, owned(&["Analog 1", "AES 1", "", "MADI 64"]));
}

#[test]
fn different_device_numbers_and_names_without_one_are_kept() {
    for original in [
        owned(&["Analog 1 (1)", "Analog 1 (2)"]),
        owned(&["Analog 1 (1)", "Loopback"]),
        owned(&["Line 1", "Line 2"]),
        owned(&["(1)", "(1)"]),
    ] {
        let mut names = original.clone();
        assert_eq!(strip_common_suffix(&mut names), None);
        assert_eq!(names, original);
    }
}

fn remembered(driver: &str, names: &[&str]) -> (String, Vec<String>) {
    (driver.to_string(), names.iter().map(|n| n.to_string()).collect())
}

#[test]
fn the_opened_driver_gets_its_own_names() {
    let r = remembered("ASIO MADIface USB", &["AN 1", "AN 2", "ADAT 3"]);
    assert_eq!(names_or_fallback(Some(&r), "ASIO MADIface USB", 3), vec!["AN 1", "AN 2", "ADAT 3"]);
}

#[test]
fn empty_and_missing_names_become_channel_n() {
    let r = remembered("ASIO MADIface USB", &["AN 1", "", "ADAT 3"]);
    assert_eq!(
        names_or_fallback(Some(&r), "ASIO MADIface USB", 5),
        vec!["AN 1", "Channel 2", "ADAT 3", "Channel 4", "Channel 5"]
    );
}

#[test]
fn other_devices_and_nothing_remembered_get_channel_n() {
    let r = remembered("ASIO MADIface USB", &["AN 1", "AN 2"]);
    let channel_n = vec!["Channel 1", "Channel 2"];
    assert_eq!(names_or_fallback(Some(&r), "ASIO4ALL v2", 2), channel_n);
    assert_eq!(names_or_fallback(Some(&r), "Speakers (RME)", 2), channel_n);
    assert_eq!(names_or_fallback(None, "ASIO MADIface USB", 2), channel_n);
}
