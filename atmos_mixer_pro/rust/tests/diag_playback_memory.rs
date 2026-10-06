//! 수동 진단(HANDOFF 6번): 재생을 오래 돌리면 Rust 엔진 메모리가 느는가.
//!
//! 앱 통합 테스트로 65분 잰 결과 RSS가 분당 2.2MB 늘었는데, 그 측정에는 Dart·3D 웹뷰·테스트 하네스가 함께
//! 돌았다. 여기서는 앱과 같은 경로(`api_init_audio_system`)로 실제 엔진만 기본 출력 장치에 띄워 가린다.
//! 엔진 감시 루프, 장치 목록 검사, 정리 스레드, 디스크 스트리밍이 앱과 같이 돈다. 트랙은 전부 무음 WAV라
//! 소리가 나지 않는다.
//! - 루프 트랙 2개를 OSC 테마 시작과 같은 `api_theme_start`로 튼다. 루프는 늘 디스크 스트리밍이다.
//!   - 테마 BGM과 같은 44.1kHz 스테레오 11.6초: 재생하면서 48kHz로 리샘플하고 11.6초마다 처음으로 돌아간다.
//!   - 48kHz 모노 30초: 리샘플 없이 30초마다 돈다.
//! - 미리 불러온 1초짜리 단발을 10초마다 다시 튼다(인스턴스 생성·정리).
//!
//! 30초마다 RSS·힙 사용량·스레드 수를 찍고, 처음 2분을 뺀 구간의 분당 증가량(최소제곱 기울기)을 낸다.
//! 자동 테스트가 아니다(#[ignore]). 실행:
//!   [DIAG_MINUTES=20] cargo test --test diag_playback_memory -- --ignored --nocapture
#![cfg(target_os = "macos")]

use rust_lib_atmos_mixer_pro::api::show::api_theme_start;
use rust_lib_atmos_mixer_pro::api::simple::{
    api_init_audio_system, api_is_engine_ready, api_play_track, api_preload_sound, api_stop_all,
    api_stop_audio_engine,
};
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::path::Path;
use std::time::{Duration, Instant};

fn ps(args: &[&str]) -> String {
    let out = std::process::Command::new("ps").args(args).output().expect("ps 실행 실패");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// (상주 메모리 MB, 스레드 수)
fn rss_and_threads() -> (f64, usize) {
    let pid = std::process::id().to_string();
    let rss_kb: f64 = ps(&["-o", "rss=", "-p", &pid]).trim().parse().unwrap_or(0.0);
    let threads = ps(&["-M", "-p", &pid]).lines().count().saturating_sub(1);
    (rss_kb / 1024.0, threads)
}

/// malloc이 지금 실제로 내준 바이트(MB). RSS와 달리 해제 후 할당자가 붙잡고 있는 공간은 빠진다.
fn heap_in_use_mb() -> f64 {
    unsafe {
        let mut stats: libc::malloc_statistics_t = std::mem::zeroed();
        libc::malloc_zone_statistics(std::ptr::null_mut(), &mut stats);
        stats.size_in_use as f64 / (1024.0 * 1024.0)
    }
}

/// 무음 16비트 WAV.
fn silent_wav(path: &Path, seconds: f32, sample_rate: u32, channels: u16) {
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for _ in 0..(sample_rate as f32 * seconds) as usize * channels as usize {
        w.write_sample(0i16).unwrap();
    }
    w.finalize().unwrap();
}

fn track(id: &str, path: &Path, is_loop: bool) -> TrackConfig {
    TrackConfig {
        id: id.into(),
        name: id.into(),
        file_path: path.to_string_lossy().into_owned(),
        volume: 1.0,
        is_loop,
        is_streaming: is_loop,
        output_channel: 0,
        output_stereo: false,
        play_osc_address: String::new(),
        stop_osc_address: String::new(),
    }
}

/// 최소제곱 기울기(y 단위/분).
fn slope_per_minute(points: &[(f64, f64)]) -> f64 {
    let n = points.len() as f64;
    let (sx, sy) = points.iter().fold((0.0, 0.0), |(a, b), (x, y)| (a + x, b + y));
    let (mx, my) = (sx / n, sy / n);
    let num: f64 = points.iter().map(|(x, y)| (x - mx) * (y - my)).sum();
    let den: f64 = points.iter().map(|(x, _)| (x - mx) * (x - mx)).sum();
    if den == 0.0 {
        0.0
    } else {
        num / den
    }
}

#[test]
#[ignore]
fn 재생을_오래_돌리면_엔진_메모리가_느는지_잰다() {
    let minutes: f64 = std::env::var("DIAG_MINUTES").ok().and_then(|v| v.parse().ok()).unwrap_or(20.0);
    let dir = std::env::temp_dir().join(format!("atmos_diag_memory_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (loop_a, loop_b, shot) = (dir.join("loop_a.wav"), dir.join("loop_b.wav"), dir.join("shot.wav"));
    silent_wav(&loop_a, 11.566, 44_100, 2);
    silent_wav(&loop_b, 30.0, 48_000, 1);
    silent_wav(&shot, 1.0, 48_000, 1);
    *GLOBAL_STATE.config.write().unwrap() = Some(AppConfig {
        rooms: vec![RoomConfig {
            id: "r1".into(),
            name: "r1".into(),
            color_hex: "#ffffff".into(),
            volume: 1.0,
            volume_osc_address: String::new(),
            clear_osc_address: String::new(),
            tracks: vec![track("la", &loop_a, true), track("lb", &loop_b, true), track("shot", &shot, false)],
        }],
        ..AppConfig::default()
    });

    api_init_audio_system(None).expect("엔진 시작 실패");
    let ready_by = Instant::now() + Duration::from_secs(10);
    while !api_is_engine_ready() {
        assert!(Instant::now() < ready_by, "엔진이 10초 안에 준비되지 않았다");
        std::thread::sleep(Duration::from_millis(50));
    }
    api_preload_sound(shot.to_string_lossy().into_owned()).unwrap();
    api_theme_start().unwrap();

    let start = Instant::now();
    let total = Duration::from_secs_f64(minutes * 60.0);
    let (rss0, th0) = rss_and_threads();
    println!("분 | RSS(MB) | 힙 사용(MB) | 스레드 | 재생 목록");
    println!("0.0 | {rss0:.1} | {:.1} | {th0} | -", heap_in_use_mb());
    let (mut rss_points, mut heap_points) = (Vec::new(), Vec::new());
    let mut next_shot = Instant::now();
    let mut next_sample = Instant::now() + Duration::from_secs(30);
    while start.elapsed() < total {
        if Instant::now() >= next_shot {
            api_play_track("r1".into(), "shot".into()).unwrap();
            next_shot += Duration::from_secs(10);
        }
        if Instant::now() >= next_sample {
            let t = start.elapsed().as_secs_f64() / 60.0;
            let (rss, th) = rss_and_threads();
            let heap = heap_in_use_mb();
            let playing = GLOBAL_STATE.playing_track_ids.read().unwrap().len();
            println!("{t:.1} | {rss:.1} | {heap:.1} | {th} | {playing}");
            if t >= 2.0 {
                rss_points.push((t, rss));
                heap_points.push((t, heap));
            }
            next_sample += Duration::from_secs(30);
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    api_stop_all().unwrap();
    api_stop_audio_engine();
    std::thread::sleep(Duration::from_millis(500));
    let _ = std::fs::remove_dir_all(&dir);

    println!(
        "2분 뒤 기울기: RSS {:+.3} MB/분, 힙 {:+.3} MB/분 (앱 통합 테스트 측정은 RSS +2.2 MB/분)",
        slope_per_minute(&rss_points),
        slope_per_minute(&heap_points)
    );
}
