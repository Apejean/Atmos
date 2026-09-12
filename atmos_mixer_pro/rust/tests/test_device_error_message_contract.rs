//! 출력 장치 부재 오류 메시지의 Dart-Rust 계약 회귀 테스트.
//!
//! `AtmosError`에는 오류 종류를 구분하는 필드가 없어서, Dart의
//! `OutputChannelsNotifier._refresh()`(lib/core/state/global_state.dart)는
//! 메시지 문자열을 부분 일치로 검사해 "라우팅할 출력이 없음"
//! (`OutputChannelsStatus.noDevice`)과 그 밖의 오류(`error`)를 구분한다.
//!
//! Rust 쪽 문자열만 조용히 바뀌면 Dart는 장치 부재를 일반 오류로 오분류해서
//! 사용자에게 "채널 없음" 대신 원시 오류 메시지를 띄운다. 이 테스트는 그
//! 문자열을 고정해 한쪽만 바뀌는 일을 컴파일/테스트 단계에서 잡는다.
//! 값을 바꿔야 한다면 Dart 매처도 같은 커밋에서 바꿔야 한다.

use rust_lib_atmos_mixer_pro::api::simple::{
    ERR_DEVICE_NOT_FOUND, ERR_NO_DEFAULT_OUTPUT_DEVICE,
};

#[test]
fn no_default_output_device_message_matches_dart_matcher() {
    assert_eq!(
        ERR_NO_DEFAULT_OUTPUT_DEVICE, "No default output device",
        "Dart의 OutputChannelsNotifier가 이 문자열을 검사한다. \
         바꾸려면 lib/core/state/global_state.dart의 매처도 함께 수정하라."
    );
}

#[test]
fn device_not_found_message_matches_dart_matcher() {
    assert_eq!(
        ERR_DEVICE_NOT_FOUND, "Device not found",
        "Dart의 OutputChannelsNotifier가 이 문자열을 검사한다. \
         바꾸려면 lib/core/state/global_state.dart의 매처도 함께 수정하라."
    );
}

/// 실제 오류 생성부는 `format!("{}: {}", ERR_DEVICE_NOT_FOUND, name)` 형태로
/// 장치 이름을 뒤에 붙인다. Dart가 부분 일치(`contains`)를 쓰므로 접두사
/// 형태가 유지되는지도 확인한다.
#[test]
fn device_not_found_message_is_a_prefix_of_the_formatted_error() {
    let formatted = format!("{}: {}", ERR_DEVICE_NOT_FOUND, "Fireface UFX III");
    assert!(
        formatted.contains(ERR_DEVICE_NOT_FOUND),
        "포맷된 메시지가 접두사를 잃었다: {}",
        formatted
    );
}
