//! 수동 진단(Windows, ASIO 장치 필요): 엔진을 ASIO 장치로 열어, 드라이버가 준 출력 채널 이름을 본다.
//! 자동 테스트가 아니다(#[ignore]). 이 드라이버를 쓰는 다른 앱(설치본 Atmos 포함)을 먼저 끈다. 무음만 낸다. 실행:
//!   [DIAG_ASIO_DEVICE="ASIO MADIface USB"] cargo test --test diag_asio_channel_names -- --ignored --nocapture
#![cfg(target_os = "windows")]

use std::sync::atomic::Ordering;

use rust_lib_atmos_mixer_pro::audio::channel_names::get_channel_names_win;
use rust_lib_atmos_mixer_pro::audio::engine::{AudioEngine, ENGINE_GENERATION};
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;

#[test]
#[ignore]
fn asio_장치의_출력_채널_이름을_본다() {
    let driver =
        std::env::var("DIAG_ASIO_DEVICE").unwrap_or_else(|_| "ASIO MADIface USB".to_string());
    let generation = ENGINE_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let (_tx, rx) = crossbeam_channel::unbounded::<AudioCommand>();
    let mut engine = AudioEngine::new();
    engine
        .start(Some(format!("[ASIO] {driver}")), rx, generation)
        .expect("엔진 시작 실패");
    let names = get_channel_names_win(&driver, 64);
    drop(engine);
    for (i, name) in names.iter().enumerate() {
        println!("CH{} = {name}", i + 1);
    }
    assert!(
        names.iter().any(|n| !n.starts_with("Channel ")),
        "드라이버가 채널 이름을 하나도 주지 않았다"
    );
}
