//! 샘플레이트를 바꾸는(리샘플) 스트리밍 파일도 멈춘 자리로 바로 찾아 이어 튼다(HANDOFF 남은 일 8의 남은 절반).
//! 예전에는 시작 위치까지 처음부터 해독·리샘플해 버려, 위치가 뒤일수록 소리가 늦게 나왔다(장치 복귀·감시 재실행·공연 이어 가기).
//! 찾은 뒤에도 소리는 처음부터 푼 것과 같아야 한다(찾는 자리를 리샘플 위상이 같은 자리로 맞춘다).
use rust_lib_atmos_mixer_pro::audio::streaming::DiskStreamer;
use std::time::{Duration, Instant};

/// `rate`Hz 모노 32비트 float WAV, 997Hz 사인 0.5(리샘플해도 값으로 맞춰 볼 수 있다).
fn sine_wav(path: &std::path::Path, rate: u32, seconds: f64) {
    let spec = hound::WavSpec { channels: 1, sample_rate: rate, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    let frames = (seconds * rate as f64) as usize;
    for i in 0..frames {
        let t = i as f64 / rate as f64;
        w.write_sample((0.5 * (2.0 * std::f64::consts::PI * 997.0 * t).sin()) as f32).unwrap();
    }
    w.finalize().unwrap();
}

/// 첫 `n`개 표본을 모은다. `limit` 안에 못 모으면 실패.
fn take(s: &mut DiskStreamer, n: usize, limit: Duration) -> Vec<f32> {
    let rx = s.chunk_receiver.as_mut().expect("수신기 없음");
    let t0 = Instant::now();
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        if let Ok(chunk) = rx.pop() {
            out.extend_from_slice(&chunk);
        } else {
            assert!(t0.elapsed() < limit, "{limit:?} 안에 표본 {n}개가 오지 않았다({}개)", out.len());
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    out.truncate(n);
    out
}

fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("atmos_rs_seek_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn 리샘플_파일도_먼_위치에서_바로_나온다() {
    let d = dir("fast");
    let path = d.join("long_44k.wav");
    sine_wav(&path, 44_100, 90.0);
    let p = path.to_string_lossy().to_string();
    let first_sound = |at: f64| {
        let t0 = Instant::now();
        let mut s = DiskStreamer::new_at(p.clone(), true, 48_000, at).unwrap();
        let _ = take(&mut s, 1, Duration::from_secs(120));
        t0.elapsed()
    };
    let near = first_sound(0.5);
    let far = first_sound(80.0);
    eprintln!("첫 소리: 0.5초 지점 {near:?}, 80초 지점 {far:?}");
    // 찾으면 위치와 상관없이 비슷하다. 처음부터 80초를 해독·리샘플해 버리면 위치만큼 늦어진다.
    assert!(
        far < near * 3 + Duration::from_millis(30),
        "80초 지점 첫 소리({far:?})가 0.5초 지점({near:?})보다 훨씬 늦다(처음부터 해독해 버리는 것으로 보인다)"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// 처음부터 푼 흐름의 `start` 프레임부터 `n`개와, `new_at(start/rate)`로 찾은 흐름의 첫 `n`개를 비교한다.
fn same_as_from_zero(src_rate: u32, out_rate: u32, is_loop: bool, start_seconds: f64, tag: &str) {
    let d = dir(tag);
    let path = d.join("sine.wav");
    sine_wav(&path, src_rate, 6.0);
    let p = path.to_string_lossy().to_string();
    let n = 4096;
    let start = (start_seconds * out_rate as f64).round() as usize;
    let mut zero = DiskStreamer::new(p.clone(), is_loop, out_rate).unwrap();
    let reference = take(&mut zero, start + n, Duration::from_secs(120));
    let mut sought = DiskStreamer::new_at(p, is_loop, out_rate, start_seconds).unwrap();
    let got = take(&mut sought, n, Duration::from_secs(120));
    let worst = reference[start..]
        .iter()
        .zip(&got)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    eprintln!("{src_rate}→{out_rate} {start_seconds}초(루프 {is_loop}): 처음부터 푼 소리와 최대 차이 {worst:e}");
    assert!(worst < 1e-3, "{src_rate}→{out_rate} {start_seconds}초: 처음부터 푼 소리와 최대 {worst} 다르다");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn 리샘플_파일의_찾은_자리는_처음부터_푼_소리와_같다_44k에서_48k() {
    same_as_from_zero(44_100, 48_000, false, 4.321, "up");
}

#[test]
fn 리샘플_파일의_찾은_자리는_처음부터_푼_소리와_같다_48k에서_44k() {
    same_as_from_zero(48_000, 44_100, false, 3.7, "down");
}

#[test]
fn 리샘플_루프의_첫_바퀴_안에서_찾은_자리도_같다() {
    same_as_from_zero(44_100, 48_000, true, 2.5, "loop");
}

#[test]
fn 리샘플_파일의_시작_부근_워밍업보다_앞도_같다() {
    same_as_from_zero(44_100, 48_000, false, 0.01, "near_start");
}

#[test]
fn 리샘플_루프의_두_번째_바퀴도_1샘플_안쪽으로_맞는다() {
    // 바퀴 안 위치로 찾으므로 바퀴마다 리샘플 위상이 조금 다를 수 있다(1샘플 안쪽). 997Hz 사인 0.5에서 반 샘플
    // 어긋남은 최대 약 0.033이라, 그보다 작으면 1샘플 안쪽이다.
    let d = dir("loop2");
    let path = d.join("sine.wav");
    sine_wav(&path, 44_100, 2.0);
    let p = path.to_string_lossy().to_string();
    let (n, start_seconds) = (4096, 3.3);
    let start = (start_seconds * 48_000f64).round() as usize;
    let mut zero = DiskStreamer::new(p.clone(), true, 48_000).unwrap();
    let reference = take(&mut zero, start + n, Duration::from_secs(120));
    let mut sought = DiskStreamer::new_at(p, true, 48_000, start_seconds).unwrap();
    let got = take(&mut sought, n, Duration::from_secs(120));
    let worst = reference[start..].iter().zip(&got).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max);
    eprintln!("루프 두 번째 바퀴 {start_seconds}초: 처음부터 푼 소리와 최대 차이 {worst:e}");
    assert!(worst < 0.034, "두 번째 바퀴에서 1샘플 넘게 어긋났다: 최대 {worst}");
    let _ = std::fs::remove_dir_all(&d);
}
