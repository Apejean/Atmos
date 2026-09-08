// tests/test_analysis_channel_width.rs
// 회귀 테스트: analysis 스레드가 프로듀서(mixer)가 실제로 링버퍼에 쓴 인터리빙 폭
// (virtual_channels)과 동일한 채널 수로 프레임 경계를 복원하는지 검증한다.
//
// 버그: engine.rs가 start_analysis_thread에 하드웨어 채널 수(config.channels, 예: 2)를
// 넘겼지만 실제 프로듀서(mixer.rs)는 virtual_channels(16채널 확장 버스) 폭으로 인터리빙된
// 데이터를 써서, 소비자(analysis.rs)가 frames = len / channels 로 프레임 경계를 잘못
// 계산해 EBU R128 LUFS와 RTA 스펙트럼이 실제 오디오와 무관한 값을 표시했다.
//
// 이 테스트는 mixer.process()가 실제로 만들어내는 것과 동일한 16채널 폭 인터리빙
// 데이터를 링버퍼에 채운 뒤, 수정된 channels 값(virtual_channels)으로
// start_analysis_thread를 호출해 LUFS/RTA가 채널 0(Left)의 1kHz 사인파를 정확히
// 복원해내는지 검증한다.

use rust_lib_atmos_mixer_pro::audio::analysis::start_analysis_thread;
use rust_lib_atmos_mixer_pro::audio::rta::RTA_FFT_SIZE;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

#[test]
fn test_analysis_thread_reconstructs_correct_frame_width_for_virtual_channels() {
    let sample_rate: u32 = 48000;
    // engine.rs의 virtual_channels = 16.max(config.channels)와 동일한 폭.
    // (예: 스테레오 하드웨어라도 mixer는 16채널 폭으로 프로듀스한다.)
    let virtual_channels: usize = 16;
    let frames: usize = sample_rate as usize; // 1초 분량

    // mixer.process()가 만드는 output 버퍼와 동일한 형태: 채널 0(L)에만
    // -6dBFS 1kHz 사인파를 싣고, 나머지 15개 가상 채널은 무음(0.0).
    let freq = 1000.0f32;
    let amp = 0.5f32;
    let mut payload = vec![0.0f32; frames * virtual_channels];
    for frame in 0..frames {
        let t = frame as f32 / sample_rate as f32;
        payload[frame * virtual_channels] =
            (2.0 * std::f32::consts::PI * freq * t).sin() * amp;
    }

    // mixer.rs의 "Send to Analysis Thread" 블록과 동일한 프로듀서 패턴으로 링버퍼에 채운다.
    let (mut producer, consumer) = rtrb::RingBuffer::<f32>::new(payload.len());
    {
        let mut chunk = producer
            .write_chunk(payload.len())
            .expect("ring buffer must accept full 1s payload");
        let (slice1, slice2) = chunk.as_mut_slices();
        let len1 = slice1.len();
        slice1.copy_from_slice(&payload[..len1]);
        if !slice2.is_empty() {
            slice2.copy_from_slice(&payload[len1..]);
        }
        chunk.commit_all();
    }

    // GLOBAL_STATE는 프로세스 전역 싱글턴이라 초기값(0.0 bits)이 이미 "무음이 아님" 조건을
    // 우연히 만족시킬 수 있다. 이번 분석 스레드가 실제로 새로 기록했는지 판별하기 위해
    // 물리적으로 나타날 수 없는 센티널 값으로 미리 리셋해 둔다.
    let sentinel_bits = f32::to_bits(-1000.0);
    GLOBAL_STATE.lufs_master[1].store(sentinel_bits, Ordering::Relaxed);
    GLOBAL_STATE.lufs_master[3].store(sentinel_bits, Ordering::Relaxed);

    // 수정 후 코드 경로: engine.rs가 넘기는 channels 인자는 이제 virtual_channels와 일치한다.
    start_analysis_thread(consumer, sample_rate, virtual_channels);

    // 분석 스레드가 백그라운드에서 링버퍼를 소비하고 GLOBAL_STATE를 갱신할 때까지 대기.
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut rta_ready = false;
    let mut magnitudes: Vec<f32> = Vec::new();
    while Instant::now() < deadline {
        if let Some(arc) = GLOBAL_STATE
            .rta_magnitudes_ref
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            let lock = arc.read();
            if lock.iter().any(|&db| db > -100.0) {
                magnitudes = lock.clone();
                rta_ready = true;
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(rta_ready, "RTA 매그니튜드가 시간 내에 갱신되지 않음 (분석 스레드 정지/데드락 의심)");

    // --- RTA 검증: 채널 0의 1kHz 사인파가 올바른 프레임 경계로 복원되어
    //     스펙트럼 피크가 1kHz 근처에 나타나야 한다. ---
    let bin_hz = sample_rate as f32 / RTA_FFT_SIZE as f32;
    let (peak_bin, _) = magnitudes
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .expect("magnitudes must not be empty");
    let detected_freq = peak_bin as f32 * bin_hz;
    println!(
        "RTA Peak: bin={} freq={:.1}Hz magnitudes[peak]={:.2}dB",
        peak_bin, detected_freq, magnitudes[peak_bin]
    );
    assert!(
        (detected_freq - freq).abs() < 50.0,
        "프레임 경계 복원 실패: 1kHz 피크가 아닌 {:.1}Hz에서 검출됨 (채널 폭 불일치 의심)",
        detected_freq
    );

    // --- LUFS 검증: 무음이 아니고, 클리핑 수준도 아닌 합리적인 라우드니스여야 한다. ---
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut short_term_bits = sentinel_bits;
    while Instant::now() < deadline {
        short_term_bits = GLOBAL_STATE.lufs_master[1].load(Ordering::Relaxed);
        if short_term_bits != sentinel_bits {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let short_term = f32::from_bits(short_term_bits);
    println!("Short-term LUFS: {:.2}", short_term);
    assert!(
        short_term > -30.0 && short_term < -3.0,
        "프레임 경계가 깨지면 LUFS가 실제 오디오와 무관해진다. Short-term LUFS = {:.2}",
        short_term
    );

    let deadline = Instant::now() + Duration::from_secs(3);
    let mut true_peak_bits = sentinel_bits;
    while Instant::now() < deadline {
        true_peak_bits = GLOBAL_STATE.lufs_master[3].load(Ordering::Relaxed);
        if true_peak_bits != sentinel_bits {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let true_peak = f32::from_bits(true_peak_bits);
    println!("True Peak (channel 0): {:.4}", true_peak);
    assert!(
        (true_peak - amp).abs() < 0.05,
        "채널 0 True Peak이 원본 -6dBFS 사인파 진폭(0.5)과 어긋남: {:.4}",
        true_peak
    );
}
