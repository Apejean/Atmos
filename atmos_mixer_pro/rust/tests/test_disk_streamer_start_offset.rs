//! 재시작 복원은 스트리밍 트랙을 멈춘 지점부터 다시 튼다: DiskStreamer가 시작 위치까지 정확히 건너뛴다.
use rust_lib_atmos_mixer_pro::audio::streaming::DiskStreamer;
use std::time::{Duration, Instant};

const FS: u32 = 48_000;

/// i번째 샘플 = i × 1e-6 인 32비트 float 모노 WAV(값으로 위치를 알 수 있다).
fn ramp_wav(path: &std::path::Path, frames: usize) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: FS,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for i in 0..frames {
        w.write_sample(i as f32 * 1e-6).unwrap();
    }
    w.finalize().unwrap();
}

fn first_sample(s: &mut DiskStreamer) -> f32 {
    let rx = s.chunk_receiver.as_mut().expect("수신기 없음");
    let t0 = Instant::now();
    loop {
        if let Ok(chunk) = rx.pop() {
            if let Some(&v) = chunk.first() {
                return v;
            }
        }
        assert!(t0.elapsed() < Duration::from_secs(3), "3초 안에 청크가 오지 않았다");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn 시작_위치만큼_건너뛰고_루프_길이를_알려준다() {
    let dir = std::env::temp_dir().join(format!("atmos_stream_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("ramp.wav");
    let frames = FS as usize * 3;
    ramp_wav(&path, frames);
    let p = path.to_string_lossy().to_string();

    let mut s0 = DiskStreamer::new(p.clone(), true, FS).unwrap();
    assert_eq!(first_sample(&mut s0), 0.0);
    assert_eq!(s0.loop_len_frames, Some(frames as f64));

    let mut s1 = DiskStreamer::new_at(p.clone(), true, FS, 1.25).unwrap();
    assert_eq!(first_sample(&mut s1), 60_000f32 * 1e-6, "1.25초(60000프레임) 지점부터 시작하지 않았다");

    // 청크 여러 개를 건너뛰는 위치도 정확해야 한다
    let mut s2 = DiskStreamer::new_at(p, false, FS, 2.999).unwrap();
    let start2 = (2.999 * FS as f64).round() as usize;
    assert_eq!(first_sample(&mut s2), start2 as f32 * 1e-6);
    let _ = std::fs::remove_dir_all(&dir);
}
