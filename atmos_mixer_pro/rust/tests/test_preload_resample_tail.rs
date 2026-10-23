//! 미리 읽는 음원(반복·스트리밍이 아닌 효과음)을 장치 샘플레이트로 바꿀 때 끝이 잘리지 않는지 본다.
//! 예전에는 1024프레임 단위로 바꾸고 마지막 자투리를 버려 끝이 최대 약 21ms(48kHz) 잘렸다(2026-10-24 기능 점검).
use rust_lib_atmos_mixer_pro::audio::player::SoundData;

fn write_mono_wav(path: &std::path::Path, sample_rate: u32, samples: &[f32]) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("시험용 WAV를 만들지 못했다");
    for s in samples {
        writer.write_sample(*s).expect("시험용 WAV에 쓰지 못했다");
    }
    writer.finalize().expect("시험용 WAV를 닫지 못했다");
}

#[test]
fn 리샘플링한_효과음은_끝까지_남고_길이가_정확하다() {
    let src_rate = 44_100u32;
    let frames = 4_410usize; // 0.1초
    let tail = 441usize; // 마지막 10ms에만 소리를 넣는다
    let samples: Vec<f32> = (0..frames)
        .map(|n| {
            if n >= frames - tail {
                0.5 * (2.0 * std::f32::consts::PI * 1000.0 * n as f32 / src_rate as f32).sin()
            } else {
                0.0
            }
        })
        .collect();
    let path = std::env::temp_dir().join(format!("atmos_preload_tail_{}.wav", std::process::id()));
    write_mono_wav(&path, src_rate, &samples);

    let loaded = SoundData::load_from_file(&path, 48_000);
    let _ = std::fs::remove_file(&path);
    let data = loaded.expect("불러오기 실패");

    assert_eq!(data.sample_rate, 48_000);
    assert_eq!(data.channels, 1);
    let out_frames = data.samples.len() / data.channels as usize;
    // 4410 × 48000 / 44100 = 4800
    assert_eq!(out_frames, 4_800, "바뀐 길이가 원래 길이 × 비율과 달라졌다");

    // 마지막 10ms(480프레임) 안에 넣었던 1kHz 소리가 남아 있어야 한다.
    let last_peak = data.samples[out_frames - 400..]
        .iter()
        .fold(0.0f32, |m, s| m.max(s.abs()));
    assert!(last_peak > 0.3, "끝의 소리가 잘렸다: 마지막 400프레임 피크 {last_peak}");
}
