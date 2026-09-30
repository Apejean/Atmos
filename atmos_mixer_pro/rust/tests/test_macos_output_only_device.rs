//! 출력 전용 장치(Apple Silicon 맥북 내장 스피커 등)가 장치 목록·채널 수 조회·
//! 엔진 기동 모든 경로에서 정상적으로 쓰이는지 검증한다.
//!
//! cpal 0.15.3은 출력 설정을 조회할 때 오디오 유닛을 입력 모드로 만들어서
//! (`audio_unit_from_device(self, true)`), 입력이 없는 장치는 `Invalid property value`로
//! 실패하고 `output_devices()`에서도 빠진다. 스트림 자체는 출력 모드로 열리므로
//! 정상 재생된다. 앱은 CoreAudio에서 직접 읽은 설정으로 이 공백을 메워야 한다.
//!
//! 하드웨어 의존 테스트다. 해당 장치가 없는 기기에서는 건너뛴다.
#![cfg(target_os = "macos")]

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rust_lib_atmos_mixer_pro::api::simple::{api_get_device_channel_count, api_get_output_devices};
use rust_lib_atmos_mixer_pro::audio::coreaudio_fallback;
use rust_lib_atmos_mixer_pro::audio::engine::AudioEngine;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn clean(name: &str) -> String {
    name.replace('\0', "").trim().to_string()
}

/// cpal `output_devices()`에는 없지만 CoreAudio는 출력 채널을 보고하는 장치 이름들.
fn devices_cpal_misses() -> Vec<String> {
    let host = cpal::default_host();
    let listed: Vec<String> = host
        .output_devices()
        .map(|ds| ds.filter_map(|d| d.name().ok()).map(|n| clean(&n)).collect())
        .unwrap_or_default();
    host.devices()
        .expect("장치 열거 실패")
        .filter_map(|d| d.name().ok())
        .map(|n| clean(&n))
        .filter(|n| !listed.contains(n))
        .filter(|n| coreaudio_fallback::output_config(n).is_some())
        .collect()
}

/// 새 조회 함수가 늘 None을 돌려주면 아래 테스트들이 전부 "장치 없음"으로 건너뛰어
/// 거짓 통과한다. 그래서 cpal이 정상적으로 읽는 장치에서는 cpal과 같은 값을 내는지
/// 먼저 확인한다(독립 오라클).
#[test]
fn cpal이_읽을_수_있는_장치에서는_직접_조회값이_cpal과_같다() {
    let host = cpal::default_host();
    let mut compared = 0;
    for d in host.output_devices().expect("출력 장치 열거 실패") {
        let Ok(name) = d.name() else { continue };
        let Ok(expected) = d.default_output_config() else { continue };
        let got = coreaudio_fallback::output_config(&clean(&name))
            .unwrap_or_else(|| panic!("'{name}': cpal은 읽는데 직접 조회는 None"));
        assert_eq!(got.channels(), expected.channels(), "'{name}' 채널 수");
        assert_eq!(got.sample_rate(), expected.sample_rate(), "'{name}' 샘플레이트");
        compared += 1;
    }
    assert!(compared > 0, "비교할 출력 장치가 하나도 없다");
}

#[test]
fn 직접_조회_설정으로_엔진과_같은_방식의_스트림이_실제로_돈다() {
    let names = devices_cpal_misses();
    if names.is_empty() {
        eprintln!("cpal이 놓치는 출력 장치가 없어 건너뜀");
        return;
    }
    let host = cpal::default_host();
    for name in names {
        let cfg = coreaudio_fallback::output_config(&name).expect("설정 없음");
        let device = host
            .devices()
            .expect("장치 열거 실패")
            .find(|d| d.name().map(|n| clean(&n) == name).unwrap_or(false))
            .expect("장치 없음");

        // 엔진(engine.rs start)과 같이 버퍼 크기를 범위 안으로 고정해서 연다.
        let buffer_size = match cfg.buffer_size() {
            cpal::SupportedBufferSize::Range { min, max } => {
                cpal::BufferSize::Fixed(2048u32.clamp(*min, *max))
            }
            cpal::SupportedBufferSize::Unknown => cpal::BufferSize::Default,
        };
        let stream_cfg = cpal::StreamConfig {
            channels: cfg.channels(),
            sample_rate: cfg.sample_rate(),
            buffer_size,
        };
        let frames = Arc::new(AtomicUsize::new(0));
        let frames_cb = frames.clone();
        let stream = device
            .build_output_stream(
                &stream_cfg,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // 무음만 출력한다(스피커에서 소리가 나지 않는다).
                    data.fill(0.0);
                    frames_cb.fetch_add(data.len(), Ordering::Relaxed);
                },
                |e| eprintln!("스트림 오류: {e}"),
                None,
            )
            .unwrap_or_else(|e| panic!("'{name}' 스트림 열기 실패: {e:?}"));
        stream.play().expect("재생 시작 실패");

        let t0 = Instant::now();
        while frames.load(Ordering::Relaxed) == 0 && t0.elapsed() < Duration::from_secs(2) {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(frames.load(Ordering::Relaxed) > 0, "'{name}' 콜백이 한 번도 오지 않았다");
    }
}

#[test]
fn 출력_전용_장치도_목록과_채널_수_조회에_나온다() {
    let names = devices_cpal_misses();
    if names.is_empty() {
        eprintln!("cpal이 놓치는 출력 장치가 없어 건너뜀");
        return;
    }
    let listed = api_get_output_devices().expect("장치 목록 조회 실패");
    for name in names {
        let expected = coreaudio_fallback::output_config(&name).unwrap().channels() as u32;
        let full = format!("[CoreAudio] {name}");
        let entry = listed
            .iter()
            .find(|d| d.name == full)
            .unwrap_or_else(|| panic!("'{full}'가 장치 목록에 없다"));
        assert_eq!(entry.max_channels, expected, "'{full}' 목록 채널 수");

        let count = api_get_device_channel_count(Some(full.clone()))
            .unwrap_or_else(|e| panic!("'{full}' 채널 수 조회 실패: {}", e.message));
        assert_eq!(count, expected, "'{full}' 채널 수 조회");
    }
}

#[test]
fn 출력_전용_장치를_이름으로_지정해_엔진을_시작할_수_있다() {
    let names = devices_cpal_misses();
    let Some(name) = names.first() else {
        eprintln!("cpal이 놓치는 출력 장치가 없어 건너뜀");
        return;
    };
    let (_tx, rx) = crossbeam_channel::unbounded();
    let mut engine = AudioEngine::new();
    let t0 = Instant::now();
    // 세대 0 = 지금 세대(ENGINE_GENERATION 초기값) — 콜백이 명령을 처리하는 정상 경로.
    let result = engine.start(Some(format!("[CoreAudio] {name}")), rx, 0);
    // 장치를 못 찾으면 엔진은 30초 동안 재시도한다. 찾았다면 즉시 끝나야 한다.
    assert!(t0.elapsed() < Duration::from_secs(10), "장치 탐색이 {:?} 걸렸다", t0.elapsed());
    assert!(result.is_ok(), "엔진 시작 실패: {:?}", result.err());
    let expected = coreaudio_fallback::output_config(name).unwrap().channels() as u32;
    let active = rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE
        .active_device_channels
        .load(Ordering::SeqCst);
    assert_eq!(active, expected, "엔진이 연 채널 수");
    drop(engine);
}
