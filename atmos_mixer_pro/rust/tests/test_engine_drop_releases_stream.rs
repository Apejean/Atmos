//! 엔진을 drop하면(워치독 재시작·엔진 정지) 그 cpal 스트림이 실제로 풀려야 한다.
//!
//! cpal 0.15.3(macOS)은 기본 장치가 아닌 장치(앱은 늘 이름으로 장치를 고른다)의 스트림에 장치
//! 분리 리스너를 달면서 리스너 안에 스트림 자신(Arc)을 넣었다. 순환 참조라 drop해도 AudioUnit·
//! 콜백·믹서가 남아 재시작마다 하나씩 쌓였고, pause가 실패하면 옛 콜백이 계속 돌며 새 엔진의
//! 명령을 가져갔다(test_stale_engine_generation 참고). 콜백이 명령 수신기를 쥐고 있으므로,
//! drop 뒤 송신이 "받는 쪽 없음"으로 실패하면 콜백과 믹서가 해제된 것이다.
//!
//! 하드웨어 의존 테스트다. 출력 장치가 없는 기기에서는 건너뛴다. 무음만 출력한다.
#![cfg(target_os = "macos")]

use cpal::traits::{DeviceTrait, HostTrait};
use rust_lib_atmos_mixer_pro::audio::engine::{AudioEngine, ENGINE_GENERATION, ENGINE_INIT_SIGNAL};
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

#[test]
fn 엔진을_drop하면_스트림이_풀려_명령_수신기도_사라진다() {
    let Some(name) = cpal::default_host()
        .default_output_device()
        .and_then(|d| d.name().ok())
    else {
        eprintln!("출력 장치가 없어 건너뜀");
        return;
    };
    let (tx, rx) = crossbeam_channel::unbounded::<AudioCommand>();
    let mut engine = AudioEngine::new();
    // 앱처럼 이름으로 고른다(cpal의 기본 장치 경로를 타지 않는다).
    let requested = format!("[CoreAudio] {}", name.replace('\0', "").trim());
    engine
        .start(Some(requested), rx, ENGINE_GENERATION.load(Ordering::SeqCst))
        .expect("엔진 시작 실패");

    let t0 = Instant::now();
    while !ENGINE_INIT_SIGNAL.load(Ordering::Acquire) && t0.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(ENGINE_INIT_SIGNAL.load(Ordering::Acquire), "콜백이 한 번도 오지 않았다");

    drop(engine);

    let t0 = Instant::now();
    let mut released = false;
    while t0.elapsed() < Duration::from_secs(2) {
        if tx.send(AudioCommand::StopTrack { room_id: 0, track_id: 0 }).is_err() {
            released = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(released, "엔진을 drop했는데 스트림 콜백이 명령 수신기를 계속 쥐고 있다(스트림 누수)");
}
