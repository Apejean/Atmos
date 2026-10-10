//! 엔진이 설정의 장치를 찾는 규칙(core::device_search, HANDOFF 남은 일 14). 기다려도 나타날 수 없는 이름은
//! 30초 동안 찾지 않는다 — Windows에서는 찾는 동안 ASIO 드라이버를 0.5초마다 모두 불러와 드라이버 창이 반복해 떴다.
use rust_lib_atmos_mixer_pro::api::simple::api_init_audio_system;
use rust_lib_atmos_mixer_pro::core::device_search::reject_before_search;
use std::time::{Duration, Instant};

/// 엔진 기동도 그런 이름이면 장치를 건드리지 않고 바로 실패한다(예전에는 30초 동안 찾았다). 장치나 드라이버를
/// 열지 않으므로 오디오 장치가 없는 CI에서도 돈다.
#[test]
fn 엔진도_열_수_없는_이름이면_바로_실패한다() {
    let names: &[&str] = if cfg!(target_os = "windows") {
        &["[CoreAudio] Scarlett 6i6 USB", "[ASIO] Atmos Test Driver Not Installed"]
    } else {
        &["[ASIO] ASIO MADIface USB"]
    };
    for name in names {
        let started = Instant::now();
        let err = api_init_audio_system(Some(name.to_string())).expect_err(name);
        assert!(started.elapsed() < Duration::from_secs(5), "{name}: {:?}", started.elapsed());
        assert!(err.message.contains("이 컴퓨터"), "{name}: {}", err.message);
    }
}

#[test]
fn 다른_os의_장치_이름은_찾지_않고_바로_실패한다() {
    let (foreign, own) = if cfg!(target_os = "windows") {
        ("[CoreAudio] Scarlett 6i6 USB", "[WASAPI] ADAT (3+4)(RME UFX+ USB 3.0)")
    } else {
        ("[ASIO] ASIO MADIface USB", "[CoreAudio] Scarlett 6i6 USB")
    };
    let reason = reject_before_search(foreign, None).expect("다른 OS의 장치 이름");
    assert!(reason.contains(foreign), "{reason}");
    assert_eq!(reject_before_search(own, None), None, "이 OS의 장치는 찾는다");
}

#[cfg(target_os = "windows")]
#[test]
fn 설치되지_않은_asio_드라이버는_찾지_않는다() {
    let installed = vec!["ASIO MADIface USB".to_string(), "Generic Low Latency ASIO Driver".to_string()];
    assert_eq!(
        reject_before_search("[ASIO] ASIO MADIface USB", Some(&installed)),
        None,
        "설치돼 있으면 기다린다(인터페이스가 늦게 켜질 수 있다)"
    );
    let reason = reject_before_search("[ASIO] Focusrite USB ASIO", Some(&installed)).expect("설치되지 않은 드라이버");
    assert!(reason.contains("Focusrite USB ASIO"), "{reason}");
    assert_eq!(reject_before_search("[ASIO] Focusrite USB ASIO", None), None, "설치 목록을 모르면 찾는다");
}
