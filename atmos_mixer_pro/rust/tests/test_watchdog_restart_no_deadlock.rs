//! 워치독 자동 재연결 경로의 구조를 소스 수준에서 고정한다.
//!
//! 실기 사고: 워치독이 발동한 뒤 오디오가 75분간 무음이었다. 원인은 엔진
//! 스레드가 **자기 자신을 join**하는 교착이었다. 엔진 스레드가 직접
//! `init_audio_system`(`api_init_audio_system`의 본체)을 부르면
//!   - 새로 띄운 스레드는 `ENGINE_THREAD`에 담긴 핸들(= 부른 쪽 스레드)을 join하고
//!   - 부른 쪽 스레드는 `rx_init.recv()`로 `engine.start()` 결과를 기다린다
//! 서로를 기다리므로 스트림이 다시 열리지 않는다(앱 로그에 `Stream:` 줄이
//! 다시 찍히지 않음, 리스캔의 `apiInitAudioSystem` await도 끝나지 않음).
//!
//! 런타임 재현에는 실제 오디오 장치가 필요해 CI에서 돌릴 수 없으므로,
//! `tests/test_no_env_lookup_in_audio_thread.rs`와 같은 정적 감사로 고정한다.

/// 감사 대상 소스를 읽는다.
fn simple_rs() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/api/simple.rs"))
        .expect("src/api/simple.rs를 읽을 수 없다")
}

#[test]
fn 자동재연결은_엔진스레드에서_직접_호출되면_안된다() {
    let src = simple_rs();

    let marker = "// If it wasn't a manual stop, auto restart";
    let start = src
        .find(marker)
        .expect("자동 재연결 블록 표식을 찾을 수 없다(블록이 사라졌거나 주석이 바뀌었다)");
    let region = &src[start..];

    let spawn_at = region.find("std::thread::spawn").expect(
        "자동 재연결이 별도 스레드에서 수행되지 않는다: \
         엔진 스레드가 ENGINE_THREAD의 자기 핸들을 join해 영구 교착된다",
    );
    let call_at = region
        .find("init_audio_system(")
        .expect("자동 재연결 블록에서 init_audio_system 호출을 찾을 수 없다");

    assert!(
        spawn_at < call_at,
        "init_audio_system이 std::thread::spawn보다 먼저 나온다 = \
         엔진 스레드가 직접 재기동을 수행한다 = 자기 join 교착(무음 사고 재발)"
    );
}

#[test]
fn 엔진_종료시_워치독_타임스탬프를_초기화해야_한다() {
    let src = simple_rs();

    let at = src
        .find("drop(engine);")
        .expect("엔진 루프 종료 지점(drop(engine))을 찾을 수 없다");
    // 종료 직후 구간만 본다. 여기서 초기화하지 않으면, 새 세대가 첫 콜백을
    // 내기 전에 워치독이 낡은 시각으로 오발해 재기동이 연쇄된다.
    let tail = &src[at..(at + 600).min(src.len())];

    assert!(
        tail.contains("watchdog_last_callback"),
        "엔진 종료 직후 watchdog_last_callback을 건드리지 않는다: 낡은 시각으로 워치독이 오발한다"
    );
    assert!(
        tail.contains(".store(0"),
        "watchdog_last_callback을 0으로 초기화하지 않는다: 낡은 시각으로 워치독이 오발한다"
    );
}
