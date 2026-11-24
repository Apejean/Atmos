//! 자체 WAV 리더(안정성 ⑨, 설계 `docs/superpowers/specs/2026-11-24-wav-reader-design.md`).
//! 27채널 이상이거나 RF64인 WAV만 자체 리더로 읽고, 그 밖은 symphonia 길 그대로다.
//! 시험 파일은 시험 안에서 만든다(저장소에 큰 파일을 넣지 않는다). 채널·프레임마다 다른 정수 값을 써서
//! 채널 순서와 변환 값을 함께 본다.

use rust_lib_atmos_mixer_pro::audio::wav_reader::{self, Choice, PcmFormat, WavReader, MAX_FILE_CHANNELS};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq)]
enum Marker {
    Riff,
    Rf64,
}

#[derive(Clone, Copy)]
struct Spec {
    marker: Marker,
    channels: u16,
    rate: u32,
    /// WAVE 형식 번호(1 PCM, 3 float, 2 ADPCM 등).
    tag: u16,
    bits: u16,
    extensible: bool,
    /// fmt 앞에 LIST·JUNK(홀수 크기) 덩어리를 둔다.
    junk_before: bool,
    frames: u32,
    /// 머리말에는 frames만큼이라고 쓰고 실제로는 이만큼 프레임만 쓴다(끊긴 파일).
    written_frames: Option<u32>,
}

impl Spec {
    fn pcm(channels: u16, bits: u16, frames: u32) -> Self {
        Spec {
            marker: Marker::Riff,
            channels,
            rate: 48_000,
            tag: 1,
            bits,
            extensible: false,
            junk_before: false,
            frames,
            written_frames: None,
        }
    }
}

/// 채널 ch(0부터)·프레임 f의 정수 값(16bit 범위). 홀수 채널은 음수 — 24bit 부호 확장도 본다.
fn int_value(ch: u16, f: u32) -> i32 {
    let v = (ch as i32 + 1) * 100 + (f % 100) as i32;
    if ch % 2 == 1 {
        -v
    } else {
        v
    }
}

/// 위 값을 읽었을 때 기대하는 f32(모든 형식에서 같다).
fn expected(ch: u16, f: u32) -> f32 {
    int_value(ch, f) as f32 / 32768.0
}

fn tmp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("atmos_test_wav_reader");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn sample_bytes(spec: &Spec, ch: u16, f: u32, out: &mut Vec<u8>) {
    let v = int_value(ch, f);
    match (spec.tag, spec.bits) {
        (3, 32) => out.extend_from_slice(&(v as f32 / 32768.0).to_le_bytes()),
        (3, 64) => out.extend_from_slice(&(v as f64 / 32768.0).to_le_bytes()),
        (_, 8) => out.push(((v >> 8) + 128) as u8),
        (_, 16) => out.extend_from_slice(&(v as i16).to_le_bytes()),
        (_, 24) => {
            let s = v * 256;
            out.extend_from_slice(&s.to_le_bytes()[..3]);
        }
        (_, 32) => out.extend_from_slice(&(v * 65536).to_le_bytes()),
        _ => out.extend(std::iter::repeat_n(0u8, (spec.bits / 8) as usize)),
    }
}

fn write_wav(path: &Path, spec: Spec) {
    let bytes_per = (spec.bits / 8) as u32;
    let block_align = spec.channels as u32 * bytes_per;
    let data_len = spec.frames * block_align;
    let written = spec.written_frames.unwrap_or(spec.frames);

    let mut fmt = Vec::new();
    let fmt_tag = if spec.extensible { 0xFFFEu16 } else { spec.tag };
    fmt.extend_from_slice(&fmt_tag.to_le_bytes());
    fmt.extend_from_slice(&spec.channels.to_le_bytes());
    fmt.extend_from_slice(&spec.rate.to_le_bytes());
    fmt.extend_from_slice(&(spec.rate * block_align).to_le_bytes());
    fmt.extend_from_slice(&(block_align as u16).to_le_bytes());
    fmt.extend_from_slice(&spec.bits.to_le_bytes());
    if spec.extensible {
        fmt.extend_from_slice(&22u16.to_le_bytes());
        fmt.extend_from_slice(&spec.bits.to_le_bytes());
        fmt.extend_from_slice(&0u32.to_le_bytes()); // 채널 마스크: 개수 확인용으로만 본다
        fmt.extend_from_slice(&spec.tag.to_le_bytes());
        fmt.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71]);
    }

    let mut body = Vec::new();
    body.extend_from_slice(b"WAVE");
    if spec.marker == Marker::Rf64 {
        body.extend_from_slice(b"ds64");
        body.extend_from_slice(&28u32.to_le_bytes());
        body.extend_from_slice(&0u64.to_le_bytes()); // RIFF 크기(읽개는 쓰지 않는다)
        body.extend_from_slice(&(data_len as u64).to_le_bytes());
        body.extend_from_slice(&(spec.frames as u64).to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes()); // 표 항목 수
    }
    if spec.junk_before {
        body.extend_from_slice(b"LIST");
        body.extend_from_slice(&4u32.to_le_bytes());
        body.extend_from_slice(b"INFO");
        body.extend_from_slice(b"JUNK");
        body.extend_from_slice(&3u32.to_le_bytes()); // 홀수 크기 + 채움 1바이트
        body.extend_from_slice(&[1, 2, 3, 0]);
    }
    body.extend_from_slice(b"fmt ");
    body.extend_from_slice(&(fmt.len() as u32).to_le_bytes());
    body.extend_from_slice(&fmt);
    body.extend_from_slice(b"data");
    let size_field = if spec.marker == Marker::Rf64 { 0xFFFF_FFFFu32 } else { data_len };
    body.extend_from_slice(&size_field.to_le_bytes());
    for f in 0..written {
        for ch in 0..spec.channels {
            sample_bytes(&spec, ch, f, &mut body);
        }
    }

    let mut file = std::fs::File::create(path).unwrap();
    match spec.marker {
        Marker::Riff => {
            file.write_all(b"RIFF").unwrap();
            file.write_all(&(body.len() as u32).to_le_bytes()).unwrap();
        }
        Marker::Rf64 => {
            file.write_all(b"RF64").unwrap();
            file.write_all(&0xFFFF_FFFFu32.to_le_bytes()).unwrap();
        }
    }
    file.write_all(&body).unwrap();
}

fn read_all(reader: &mut WavReader, block: usize) -> Vec<f32> {
    let mut all = Vec::new();
    let mut buf = Vec::new();
    loop {
        let n = reader.read(block, &mut buf).unwrap();
        if n == 0 {
            break;
        }
        assert_eq!(buf.len(), n * reader.info().channels as usize);
        all.extend_from_slice(&buf);
    }
    all
}

fn assert_values(all: &[f32], channels: u16, frames: u32) {
    assert_eq!(all.len(), channels as usize * frames as usize);
    for f in 0..frames {
        for ch in 0..channels {
            let got = all[f as usize * channels as usize + ch as usize];
            assert_eq!(got, expected(ch, f), "frame {f} channel {ch}");
        }
    }
}

fn own(path: &Path) -> wav_reader::WavInfo {
    match wav_reader::choose(path) {
        Choice::Own(info) => info,
        Choice::Symphonia => panic!("{} went to symphonia", path.display()),
        Choice::Unsupported(msg) => panic!("{} unsupported: {msg}", path.display()),
    }
}

#[test]
fn files_symphonia_can_open_stay_on_symphonia() {
    let p = tmp("ch26.wav");
    write_wav(&p, Spec::pcm(26, 16, 10));
    assert!(matches!(wav_reader::choose(&p), Choice::Symphonia));
    let p = tmp("stereo.wav");
    write_wav(&p, Spec::pcm(2, 24, 10));
    assert!(matches!(wav_reader::choose(&p), Choice::Symphonia));
    // WAV가 아닌 파일(예: MP3)은 머리말을 보지 않고 symphonia로.
    let p = tmp("not_wav.mp3");
    std::fs::write(&p, b"ID3\x04\x00\x00\x00\x00\x00\x00 not a wave file").unwrap();
    assert!(matches!(wav_reader::choose(&p), Choice::Symphonia));
}

#[test]
fn reads_27_and_more_channels_in_file_order_for_each_format() {
    for (name, tag, bits, format) in [
        ("ch32_i16.wav", 1u16, 16u16, PcmFormat::I16),
        ("ch32_i24.wav", 1, 24, PcmFormat::I24),
        ("ch32_i32.wav", 1, 32, PcmFormat::I32),
        ("ch32_f32.wav", 3, 32, PcmFormat::F32),
    ] {
        let p = tmp(name);
        write_wav(&p, Spec { tag, ..Spec::pcm(32, bits, 250) });
        let info = own(&p);
        assert_eq!(info.channels, 32, "{name}");
        assert_eq!(info.frames, 250, "{name}");
        assert_eq!(info.sample_rate, 48_000, "{name}");
        assert_eq!(info.format, format, "{name}");
        assert!(!info.rf64);
        let mut r = WavReader::open(&p).unwrap();
        assert_values(&read_all(&mut r, 64), 32, 250);
    }
}

#[test]
fn reads_extensible_header_with_chunks_before_fmt() {
    let p = tmp("ch64_ext_junk.wav");
    write_wav(&p, Spec { extensible: true, junk_before: true, ..Spec::pcm(64, 24, 120) });
    let info = own(&p);
    assert_eq!(info.channels, 64);
    assert_eq!(info.format, PcmFormat::I24);
    let mut r = WavReader::open(&p).unwrap();
    assert_values(&read_all(&mut r, 1024), 64, 120);

    let p = tmp("ch40_ext_float.wav");
    write_wav(&p, Spec { extensible: true, tag: 3, ..Spec::pcm(40, 32, 30) });
    assert_eq!(own(&p).format, PcmFormat::F32);
}

#[test]
fn reads_rf64_at_any_channel_count() {
    for channels in [2u16, 32] {
        let p = tmp(&format!("rf64_{channels}.wav"));
        write_wav(&p, Spec { marker: Marker::Rf64, ..Spec::pcm(channels, 24, 300) });
        let info = own(&p);
        assert!(info.rf64);
        assert_eq!(info.channels, channels);
        assert_eq!(info.frames, 300, "data 크기는 ds64에서 읽는다");
        let mut r = WavReader::open(&p).unwrap();
        assert_values(&read_all(&mut r, 77), channels, 300);
    }
}

#[test]
fn seek_lands_on_the_exact_frame() {
    let p = tmp("seek_ch32.wav");
    write_wav(&p, Spec::pcm(32, 24, 500));
    let mut r = WavReader::open(&p).unwrap();
    let all = read_all(&mut r, 100);
    for start in [0u64, 1, 99, 250, 499] {
        r.seek(start).unwrap();
        let rest = read_all(&mut r, 33);
        assert_eq!(rest, all[start as usize * 32..], "seek {start}");
    }
    r.seek(500).unwrap();
    let mut buf = Vec::new();
    assert_eq!(r.read(10, &mut buf).unwrap(), 0, "끝으로 찾으면 바로 끝");
}

#[test]
fn a_cut_off_file_reads_what_is_there() {
    let p = tmp("cut_ch32.wav");
    write_wav(&p, Spec { written_frames: Some(600), ..Spec::pcm(32, 16, 1000) });
    let info = own(&p);
    assert_eq!(info.frames, 600, "머리말보다 짧은 파일은 실제 길이까지");
    let mut r = WavReader::open(&p).unwrap();
    assert_values(&read_all(&mut r, 128), 32, 600);
}

#[test]
fn unsupported_big_files_are_refused_with_a_reason() {
    for (name, spec, word) in [
        ("f64_ch32.wav", Spec { tag: 3, ..Spec::pcm(32, 64, 10) }, "64"),
        ("u8_ch32.wav", Spec::pcm(32, 8, 10), "8"),
        ("adpcm_ch32.wav", Spec { tag: 2, ..Spec::pcm(32, 16, 10) }, "형식"),
        ("too_many.wav", Spec::pcm(MAX_FILE_CHANNELS + 1, 16, 2), "256"),
    ] {
        let p = tmp(name);
        write_wav(&p, spec);
        match wav_reader::choose(&p) {
            Choice::Unsupported(msg) => assert!(msg.contains(word), "{name}: {msg}"),
            Choice::Own(_) => panic!("{name} should be refused"),
            Choice::Symphonia => panic!("{name} went to symphonia"),
        }
        assert!(WavReader::open(&p).is_err(), "{name}");
    }
}

#[test]
fn reading_reuses_the_callers_buffer() {
    let p = tmp("reuse_ch32.wav");
    write_wav(&p, Spec::pcm(32, 16, 300));
    let mut r = WavReader::open(&p).unwrap();
    let mut buf = Vec::with_capacity(32 * 100);
    let cap = buf.capacity();
    let n = r.read(100, &mut buf).unwrap();
    assert_eq!(n, 100);
    assert_eq!(buf.capacity(), cap, "충분한 버퍼면 다시 잡지 않는다");
    let n = r.read(50, &mut buf).unwrap();
    assert_eq!((n, buf.len()), (50, 50 * 32), "버퍼를 비우고 그만큼만 채운다");
}

// ---- 2단계: 채널 수·길이·메모리 읽기·미리 읽기 한도(D3) ----

use rust_lib_atmos_mixer_pro::audio::player::{self, SoundData};
use rust_lib_atmos_mixer_pro::audio::streaming;

#[test]
fn channel_count_length_and_memory_load_use_the_own_reader() {
    let p = tmp("ch32_load.wav");
    write_wav(&p, Spec::pcm(32, 24, 480));
    // 예전에는 열기 실패로 2(스테레오)를 돌려줘 화면·처리 폭이 틀렸다.
    assert_eq!(SoundData::probe_channels(&p), 32);
    assert_eq!(streaming::file_frames(&p.to_string_lossy()), Some((480, 48_000)));
    let (data, src) = SoundData::load_from_file_with_source(&p, 48_000).unwrap();
    assert_eq!((data.channels, data.sample_rate, src), (32, 48_000, 48_000));
    assert_values(&data.samples, 32, 480);
    // 장치가 44.1kHz면 지금과 같은 리샘플 길로 바뀐다(길이는 비율만큼).
    let (data, src) = SoundData::load_from_file_with_source(&p, 44_100).unwrap();
    assert_eq!((data.channels, data.sample_rate, src), (32, 44_100, 48_000));
    assert_eq!(data.samples.len(), (480.0f64 * 44_100.0 / 48_000.0).round() as usize * 32);
}

#[test]
fn rf64_length_and_channels_are_known() {
    let p = tmp("rf64_len.wav");
    write_wav(&p, Spec { marker: Marker::Rf64, ..Spec::pcm(2, 16, 300) });
    assert_eq!(SoundData::probe_channels(&p), 2);
    assert_eq!(streaming::file_frames(&p.to_string_lossy()), Some((300, 48_000)));
}

#[test]
fn an_unsupported_big_file_fails_to_load_with_the_reason() {
    let p = tmp("f64_load.wav");
    write_wav(&p, Spec { tag: 3, ..Spec::pcm(32, 64, 10) });
    let err = SoundData::load_from_file_with_source(&p, 48_000).err().expect("거절해야 한다");
    assert!(err.to_string().contains("64"), "{err}");
}

/// 머리말만 쓰고 파일 길이를 늘린 큰 파일(값은 0). 실제로 쓰지 않아 빠르다.
fn big_empty_wav(name: &str, channels: u16, frames: u32) -> PathBuf {
    let p = tmp(name);
    write_wav(&p, Spec { written_frames: Some(0), ..Spec::pcm(channels, 16, frames) });
    let header = std::fs::metadata(&p).unwrap().len();
    let f = std::fs::OpenOptions::new().write(true).open(&p).unwrap();
    f.set_len(header + channels as u64 * 2 * frames as u64).unwrap();
    p
}

#[test]
fn files_too_big_for_memory_play_from_disk() {
    // 64채널 × 40만 프레임 = 2천5백6십만 샘플 > 미리 읽기 한도(2천5백만, 약 100MB).
    let big = big_empty_wav("big_ch64.wav", 64, 400_000);
    assert_eq!(player::estimated_samples(&big, 48_000), Some(25_600_000));
    assert!(player::too_big_for_memory(&big, 48_000));
    assert!(player::plays_from_disk(false, false, &big, 48_000), "스트리밍을 꺼 둔 단발도 디스크에서");

    let small = tmp("small_ch32.wav");
    write_wav(&small, Spec::pcm(32, 16, 100));
    assert_eq!(player::estimated_samples(&small, 48_000), Some(3_200));
    assert!(!player::plays_from_disk(false, false, &small, 48_000), "작은 단발은 지금처럼 메모리");
    assert!(player::plays_from_disk(true, false, &small, 48_000), "루프는 늘 디스크");
    assert!(player::plays_from_disk(false, true, &small, 48_000), "스트리밍 켬은 디스크");

    // 26채널 이하(symphonia 길) 파일도 같은 규칙: 리샘플이면 바뀐 길이로 센다.
    let stereo = tmp("stereo_est.wav");
    write_wav(&stereo, Spec::pcm(2, 16, 4_800));
    assert_eq!(player::estimated_samples(&stereo, 48_000), Some(9_600));
    assert_eq!(player::estimated_samples(&stereo, 44_100), Some(8_820));
}

// ---- 3단계: 스트리밍(디코더 스레드) ----

use rust_lib_atmos_mixer_pro::audio::streaming::DiskStreamer;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

/// 링에서 `samples`개가 모일 때까지 꺼낸다.
fn collect(s: &mut DiskStreamer, samples: usize) -> Vec<f32> {
    let mut rx = s.chunk_receiver.take().unwrap();
    let mut out = Vec::new();
    let t0 = Instant::now();
    while out.len() < samples {
        match rx.pop() {
            Ok(c) => {
                assert_eq!(c.len() % s.channels as usize, 0, "묶음이 프레임 단위가 아니다");
                out.extend_from_slice(&c);
            }
            Err(_) => {
                assert!(t0.elapsed() < Duration::from_secs(5), "묶음이 오지 않는다({}개 모임)", out.len());
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
    s.chunk_receiver = Some(rx);
    out
}

#[test]
fn streams_a_32_channel_loop_across_the_loop_end() {
    let p = tmp("stream_loop_ch32.wav");
    write_wav(&p, Spec::pcm(32, 24, 2_400));
    let mut s = DiskStreamer::new(p.to_string_lossy().to_string(), true, 48_000).unwrap();
    assert_eq!((s.channels, s.sample_rate, s.source_sample_rate), (32, 48_000, 48_000));
    assert_eq!(s.loop_len_frames, Some(2_400.0));
    let got = collect(&mut s, 32 * 6_000);
    for f in 0..6_000u32 {
        for ch in 0..32u16 {
            assert_eq!(got[f as usize * 32 + ch as usize], expected(ch, f % 2_400), "frame {f} ch {ch}");
        }
    }
}

#[test]
fn a_stream_starts_at_the_requested_frame() {
    let p = tmp("stream_at_ch32.wav");
    write_wav(&p, Spec::pcm(32, 16, 4_800));
    let mut s = DiskStreamer::new_at(p.to_string_lossy().to_string(), false, 48_000, 1_234.0 / 48_000.0).unwrap();
    let got = collect(&mut s, 32 * 100);
    for i in 0..100u32 {
        for ch in 0..32u16 {
            assert_eq!(got[i as usize * 32 + ch as usize], expected(ch, 1_234 + i), "frame {} ch {ch}", 1_234 + i);
        }
    }
}

#[test]
fn a_resampled_stream_keeps_every_channel() {
    let p = tmp("stream_rs_ch32.wav");
    write_wav(&p, Spec::pcm(32, 16, 4_800));
    let mut s = DiskStreamer::new(p.to_string_lossy().to_string(), false, 44_100).unwrap();
    assert_eq!((s.channels, s.sample_rate, s.source_sample_rate), (32, 44_100, 48_000));
    let got = collect(&mut s, 32 * 2_048);
    // 리샘플 뒤에도 채널마다 부호가 그대로다(홀수 채널은 음수 값만 썼다) — 채널이 섞이지 않았다.
    let mid = 1_000usize;
    for ch in 0..32usize {
        let v = got[mid * 32 + ch];
        assert!(if ch % 2 == 1 { v < 0.0 } else { v > 0.0 }, "ch {ch} = {v}");
    }
}

#[test]
fn an_own_reader_loop_waits_for_a_missing_file_and_picks_up_again() {
    let p = tmp("stream_reopen_ch32.wav");
    let moved = tmp("stream_reopen_ch32_moved.wav");
    let _ = std::fs::remove_file(&moved);
    write_wav(&p, Spec::pcm(32, 16, 1_200));
    let mut s = DiskStreamer::new(p.to_string_lossy().to_string(), true, 48_000).unwrap();
    let t0 = Instant::now();
    while !s.first_chunk_ready.load(Ordering::Acquire) {
        assert!(t0.elapsed() < Duration::from_secs(3));
        std::thread::sleep(Duration::from_millis(2));
    }
    // 디코더는 이미 열었다. 다음 바퀴에서 다시 열 때 실패하게 치운다.
    std::fs::rename(&p, &moved).unwrap();
    let mut rx = s.chunk_receiver.take().unwrap();
    let mut values = Vec::new();
    let mut idle = Instant::now();
    loop {
        match rx.pop() {
            Ok(c) => {
                values.extend_from_slice(&c);
                idle = Instant::now();
            }
            Err(_) => {
                if idle.elapsed() > Duration::from_millis(400) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        }
        assert!(t0.elapsed() < Duration::from_secs(6), "다시 열기가 실패하지 않았다");
    }
    assert!(!rx.is_abandoned(), "다시 열기에 실패하자 디코더 스레드가 끝났다");
    let frames = values.len() / 32;
    assert_eq!(frames % 1_200, 0, "바퀴 끝까지 보내고 멈춰야 한다");
    std::fs::rename(&moved, &p).unwrap();
    let back = Instant::now();
    let first = loop {
        if let Ok(c) = rx.pop() {
            break c;
        }
        assert!(back.elapsed() < Duration::from_secs(4), "파일이 돌아왔는데 다시 열지 않는다");
        std::thread::sleep(Duration::from_millis(5));
    };
    for ch in 0..32u16 {
        assert_eq!(first[ch as usize], expected(ch, 0), "다음 바퀴 처음부터, ch {ch}");
    }
    s.chunk_receiver = Some(rx);
}
