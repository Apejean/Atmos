//! 수동 진단: 워치독 재시작(세대 올림 → 옛 엔진 drop → 새 엔진 start)을 반복하면 무엇이 쌓이는가.
//!
//! 재시작마다 표로 찍는다: 아직 명령을 받을 수 있는 옛 엔진 수(= 풀리지 않은 옛 스트림), 상주
//! 메모리(RSS), 힙 사용량, 스레드 수, 직후 1초 동안의 CPU 사용률. cpal 0.15.3(macOS)에서는 옛 스트림이
//! 재시작마다 하나씩 남았다(test_engine_drop_releases_stream 참고).
//!
//! 자동 테스트가 아니다(#[ignore]). 기본 출력 장치를 이름으로 연다(무음만 낸다). 실행:
//!   [DIAG_RESTARTS=10] cargo test --test diag_engine_restart_leak -- --ignored --nocapture
#![cfg(target_os = "macos")]

use cpal::traits::{DeviceTrait, HostTrait};
use rust_lib_atmos_mixer_pro::audio::engine::{AudioEngine, ENGINE_GENERATION};
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use std::sync::atomic::Ordering;
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

/// 이 프로세스가 지금까지 쓴 사용자+시스템 CPU 시간.
fn cpu_time() -> Duration {
    unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        assert_eq!(libc::getrusage(libc::RUSAGE_SELF, &mut usage), 0);
        let u = Duration::new(usage.ru_utime.tv_sec as u64, (usage.ru_utime.tv_usec as u32) * 1000);
        let s = Duration::new(usage.ru_stime.tv_sec as u64, (usage.ru_stime.tv_usec as u32) * 1000);
        u + s
    }
}

#[test]
#[ignore]
fn 재시작마다_남는_것을_잰다() {
    let name = cpal::default_host()
        .default_output_device()
        .and_then(|d| d.name().ok())
        .expect("출력 장치가 없다");
    let requested = format!("[CoreAudio] {}", name.replace('\0', "").trim());
    let restarts: usize =
        std::env::var("DIAG_RESTARTS").ok().and_then(|v| v.parse().ok()).unwrap_or(10);

    let mut senders: Vec<crossbeam_channel::Sender<AudioCommand>> = Vec::new();
    let mut engine: Option<AudioEngine> = None;
    let (rss0, th0) = rss_and_threads();
    println!("재시작 | 살아 있는 옛 엔진 | RSS(MB) | 힙 사용(MB) | 스레드 | CPU(1초)");
    println!("0 | 0 | {rss0:.1} | {:.1} | {th0} | -", heap_in_use_mb());
    for i in 1..=restarts {
        let generation = ENGINE_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
        drop(engine.take());
        let (tx, rx) = crossbeam_channel::unbounded::<AudioCommand>();
        let mut e = AudioEngine::new();
        e.start(Some(requested.clone()), rx, generation).expect("엔진 시작 실패");
        engine = Some(e);
        std::thread::sleep(Duration::from_millis(300));

        let alive_old = senders
            .iter()
            .filter(|t| t.send(AudioCommand::StopTrack { room_id: 0, track_id: 0 }).is_ok())
            .count();
        senders.push(tx);

        let (c0, w0) = (cpu_time(), Instant::now());
        std::thread::sleep(Duration::from_secs(1));
        let cpu = (cpu_time() - c0).as_secs_f64() / w0.elapsed().as_secs_f64() * 100.0;
        let (rss, th) = rss_and_threads();
        println!("{i} | {alive_old} | {rss:.1} | {:.1} | {th} | {cpu:.1}%", heap_in_use_mb());
    }
    drop(engine);
}
