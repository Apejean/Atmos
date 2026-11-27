//! 음원 파일 채널 수 확인이 느린 파일에 막혀 앱 시작을 붙잡지 않는지 본다.
//!
//! 2026-11-28 Mac: 시작 때 설정을 읽는 `api_get_config` → 출력 채널 열기 → `cached_file_channels`가 데스크톱 폴더
//! 음원의 머리를 읽다가 macOS 파일 접근 확인에 걸려 `read`가 60~70초 멈췄다. 그동안 기억 잠금을 쥐고 있어 다른 FRB
//! 작업도 기다렸고, 시작 화면에서 엔진·OSC·공연 이어 가기가 모두 늦었다. 현장에서도 잠든 USB 디스크나 네트워크 경로의
//! 음원이면 같은 식으로 막힐 수 있다.
//!
//! 느린 파일은 FIFO(이름 있는 파이프)로 흉내 낸다. 쓰는 쪽이 열 때까지 읽기용 열기가 멈춘다.
//! FIFO는 유닉스에만 있어 Windows에서는 이 파일을 빼고 컴파일한다.
#![cfg(unix)]

use rust_lib_atmos_mixer_pro::core::output_groups::{cached_file_channels, FILE_CHANNELS_PROBE_WAIT};
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("atmos_probe_{}_{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("임시 폴더");
    dir
}

fn make_fifo(path: &Path) {
    let c = CString::new(path.to_str().expect("경로")).expect("경로");
    assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o644) }, 0, "mkfifo 실패");
}

/// 48kHz 16bit PCM, `channels`채널 무음 0.1초.
fn write_wav(path: &Path, channels: u16) {
    let frames = 4800u32;
    let block = channels as u32 * 2;
    let data_len = frames * block;
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&channels.to_le_bytes());
    b.extend_from_slice(&48_000u32.to_le_bytes());
    b.extend_from_slice(&(48_000 * block).to_le_bytes());
    b.extend_from_slice(&(block as u16).to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    b.resize(b.len() + data_len as usize, 0);
    std::fs::write(path, b).expect("wav 쓰기");
}

/// 쓰는 쪽을 열었다 닫아 막힌 읽기를 풀어 준다(읽는 쪽은 바로 끝을 만난다).
fn release_fifo(path: &Path) {
    let _ = std::fs::OpenOptions::new().write(true).open(path);
}

#[test]
fn slow_file_does_not_hold_up_the_caller_or_other_files() {
    let dir = temp_dir("slow");
    let slow = dir.join("slow.wav");
    make_fifo(&slow);
    let normal = dir.join("normal_4ch.wav");
    write_wav(&normal, 4);

    // 느린 파일: 정해진 시간만 기다리고 모른다(None)고 돌려준다.
    let slow_path = slow.to_str().expect("경로").to_string();
    let started = Instant::now();
    let waiter = std::thread::spawn(move || cached_file_channels(&slow_path));
    // 느린 파일을 확인하는 동안 다른 파일은 기다리지 않는다.
    std::thread::sleep(Duration::from_millis(100));
    let t = Instant::now();
    assert_eq!(cached_file_channels(normal.to_str().expect("경로")), Some(4));
    assert!(
        t.elapsed() < Duration::from_millis(500),
        "느린 파일을 확인하는 동안 다른 파일 확인이 {:?} 기다렸다",
        t.elapsed()
    );

    // 쓰는 쪽이 끝내 오지 않는 경우를 보려고, 기다림 한도보다 넉넉히 늦게 풀어 준다(시험이 끝나도 스레드가 남지 않게).
    let release_path = slow.clone();
    let releaser = std::thread::spawn(move || {
        std::thread::sleep(FILE_CHANNELS_PROBE_WAIT * 3);
        release_fifo(&release_path);
    });
    let result = waiter.join().expect("확인 스레드");
    let waited = started.elapsed();
    assert_eq!(result, None, "읽지 못한 파일의 채널 수를 안다고 했다");
    assert!(
        waited < FILE_CHANNELS_PROBE_WAIT + Duration::from_millis(700),
        "느린 파일 하나에 {:?} 붙잡혔다(한도 {:?})",
        waited,
        FILE_CHANNELS_PROBE_WAIT
    );

    // 아직 확인 중인 파일을 다시 물으면 또 기다리지 않는다.
    let t = Instant::now();
    assert_eq!(cached_file_channels(slow.to_str().expect("경로")), None);
    assert!(t.elapsed() < Duration::from_millis(200), "확인 중인 파일을 다시 기다렸다: {:?}", t.elapsed());

    releaser.join().expect("풀기 스레드");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn late_probe_result_is_remembered() {
    let dir = temp_dir("late");
    let late = dir.join("late.wav");
    make_fifo(&late);
    let late_path = late.to_str().expect("경로").to_string();

    // 한도 안에 끝나지 않는 확인은 None.
    assert_eq!(cached_file_channels(&late_path), None);
    // FIFO를 풀면 뒤에서 확인이 끝난다(머리말이 없어 채널 수는 여전히 모름).
    release_fifo(&late);
    std::thread::sleep(Duration::from_millis(300));
    // 끝난 뒤에는 묻는 즉시 돌려준다.
    let t = Instant::now();
    let _ = cached_file_channels(&late_path);
    assert!(t.elapsed() < Duration::from_millis(200), "끝난 확인을 다시 기다렸다: {:?}", t.elapsed());
    let _ = std::fs::remove_dir_all(&dir);
}
