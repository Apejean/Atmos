//! 비상 전환 뒤 원래 장치로 돌아가기(core::device_return).
//!
//! 실기(2026-10-07, Scarlett 6i6 12ch): 케이블을 30초 넘게 뽑으면 엔진이 기본 장치(맥북 스피커)로 비상 전환하고
//! 출력에 −40dB를 건다. 다시 꽂으면 엔진이 스스로 다시 시작하지만, 비상 엔진은 장치 이름 없이 열려 있어서 다시 열 때도
//! 이름 없이 열렸고 감쇠가 풀리지 않았다(시험음이 거의 안 들림). 사람이 환경설정에서 장치를 다시 골라야 했다.
//! 이 시험은 자기 재시작 때 어느 장치로 열지 정하는 규칙을 고정한다.

use rust_lib_atmos_mixer_pro::core::device_return::{
    device_for_self_restart, remember_requested_device, requested_device,
};

const SCARLETT: &str = "[CoreAudio] Scarlett 6i6 USB";
const MACBOOK: &str = "[CoreAudio] MacBook Pro 스피커";

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn 비상_상태가_아니면_이번_엔진을_연_장치로_다시_연다() {
    // 장치가 목록에 없어도 그대로다 — 엔진이 30초 동안 찾다가 못 찾으면 비상 전환한다(짧은 유실은 이 길로 돌아온다).
    assert_eq!(
        device_for_self_restart(Some(SCARLETT), false, Some(SCARLETT), &names(&[MACBOOK])),
        Some(SCARLETT.to_string())
    );
    assert_eq!(device_for_self_restart(None, false, None, &names(&[MACBOOK])), None);
}

#[test]
fn 비상_상태에서_설정의_장치가_돌아왔으면_그_장치를_지정해_연다() {
    assert_eq!(
        device_for_self_restart(None, true, Some(SCARLETT), &names(&[MACBOOK, SCARLETT])),
        Some(SCARLETT.to_string())
    );
}

#[test]
fn 비상_상태에서_설정의_장치가_아직_없으면_기본_장치에_머문다() {
    // 없는 장치를 지정하면 엔진이 30초 동안 찾느라 그동안 소리가 아예 안 난다.
    assert_eq!(device_for_self_restart(None, true, Some(SCARLETT), &names(&[MACBOOK])), None);
}

#[test]
fn 기본_장치를_고른_설정이면_비상_상태여도_기본_장치로_연다() {
    assert_eq!(device_for_self_restart(None, true, None, &names(&[MACBOOK, SCARLETT])), None);
    assert_eq!(device_for_self_restart(None, true, Some(""), &names(&[MACBOOK, ""])), None);
}

#[test]
fn 장치_이름은_엔진_검색처럼_호스트_표시와_널_문자와_앞뒤_공백을_빼고_비교한다() {
    // 돌려주는 값은 설정의 이름 그대로다(호스트 표시로 ASIO/WASAPI를 고르므로).
    assert_eq!(
        device_for_self_restart(None, true, Some(SCARLETT), &names(&["Scarlett 6i6 USB\0 "])),
        Some(SCARLETT.to_string())
    );
    assert_eq!(
        device_for_self_restart(None, true, Some("[ASIO] Focusrite USB ASIO"), &names(&["[ASIO] Focusrite USB ASIO"])),
        Some("[ASIO] Focusrite USB ASIO".to_string())
    );
    assert_eq!(
        device_for_self_restart(None, true, Some(SCARLETT), &names(&["[CoreAudio] Scarlett 18i8 USB"])),
        None
    );
}

#[test]
fn 설정의_장치_기억은_마지막_요청을_따른다() {
    remember_requested_device(Some(SCARLETT.to_string()));
    assert_eq!(requested_device(), Some(SCARLETT.to_string()));
    remember_requested_device(None);
    assert_eq!(requested_device(), None);
}
