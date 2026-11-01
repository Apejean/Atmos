use rtrb::Consumer;
use std::sync::atomic::Ordering;
use crate::core::state::GLOBAL_STATE;
use crate::audio::rta::RtaAnalyzer;

/// 마스터 LUFS 측정기(순간·단기·통합 음량, 트루 피크).
///
/// 통합 음량은 히스토그램으로 잰다(칸 1000개 고정). 기본(큐) 방식은 켠 뒤의 0.4초 블록을 모두 쌓고
/// 물을 때마다 전부 다시 훑어서, 이 스레드가 오디오마다 묻는 동안 메모리와 CPU가 켠 시간에 비례해
/// 늘었다(2026-11-01 Windows 28시간 시험: 앱 CPU 0.90→1.01코어, 메모리 분당 약 0.005MB).
fn new_lufs_meter(channels: u32, sample_rate: u32) -> Option<ebur128::EbuR128> {
    ebur128::EbuR128::new(
        channels,
        sample_rate,
        ebur128::Mode::M
            | ebur128::Mode::S
            | ebur128::Mode::I
            | ebur128::Mode::TRUE_PEAK
            | ebur128::Mode::HISTOGRAM,
    )
    .ok()
}

pub fn start_analysis_thread(
    mut rx: Consumer<f32>,
    sample_rate: u32,
    channels: usize,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        // 무음이 이어지면 LUFS·RTA 필터 상태도 비정규 실수가 되어 느려진다(audio::denormal 참고).
        // 이 스레드 전용이라 스레드가 끝날 때까지 켜 둔다.
        let _denormals = crate::audio::denormal::DenormalGuard::new();
        let mut rta_analyzer = RtaAnalyzer::new();
        {
            let mut lock = GLOBAL_STATE.rta_magnitudes_ref.write().unwrap_or_else(|e| e.into_inner());
            *lock = Some(rta_analyzer.get_magnitudes_arc());
        }

        let mut lufs_meter = new_lufs_meter(channels.max(1) as u32, sample_rate);

        let mut temp_buffer = Vec::with_capacity(16384);
        let mut mono_mix = Vec::with_capacity(16384);

        let ch_count = channels.max(1);

        loop {
            // rtrb의 read_chunk(0)은 "0슬롯 읽기"로 **성공**한다. 예전에는
            // read_chunk(rx.slots())를 그대로 호출해서, 링버퍼가 비어 있어도
            // Ok 분기로 들어가 빈 버퍼로 ebur128 loudness 질의를 돌린 뒤 sleep
            // 없이 곧바로 다시 돌았다. 5ms sleep이 있는 else는 사실상 죽은
            // 코드였고, 결과적으로 무음 상태에서도 이 스레드가 코어 하나를
            // 100% 태웠다. 실시간 오디오 콜백과 CPU를 다투게 되어 드롭아웃
            // (딱딱거리는 잡음)의 원인이 된다.
            //
            // 이제 데이터가 실제로 있을 때만 처리하고, 없으면 반드시 쉰다.
            let slots = rx.slots();

            // 인터리빙 프레임 경계에 맞춰 읽는다. 프레임 중간에서 끊어 읽으면
            // 남은 샘플이 버려지면서 이후 청크의 채널 정렬이 밀려, LUFS/RTA가
            // 채널을 뒤섞어 계산한다.
            let usable = (slots / ch_count) * ch_count;

            if usable == 0 {
                // 엔진은 시작할 때마다 이 스레드를 새로 띄운다. 생산자(엔진의 믹서)가 사라졌는데
                // 끝내지 않으면 워치독 재시작마다 5ms마다 깨어나는 스레드가 하나씩 쌓인다.
                if rx.is_abandoned() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
                continue;
            }

            if let Ok(chunk) = rx.read_chunk(usable) {
                let (slice1, slice2) = chunk.as_slices();
                temp_buffer.clear();
                temp_buffer.extend_from_slice(slice1);
                temp_buffer.extend_from_slice(slice2);
                chunk.commit_all();

                let frames = temp_buffer.len() / channels.max(1);
                
                // --- RTA Processing ---
                mono_mix.clear();
                for frame in 0..frames {
                    let mut sum = 0.0;
                    let mut count = 0;
                    for ch in 0..channels.min(2) {
                        if GLOBAL_STATE.enabled_channels[ch].load(Ordering::Relaxed) {
                            sum += temp_buffer[frame * channels + ch];
                            count += 1;
                        }
                    }
                    if count > 0 {
                        mono_mix.push(sum / count as f32);
                    } else {
                        mono_mix.push(0.0);
                    }
                }
                rta_analyzer.process_samples(&mono_mix);

                // --- LUFS Metering ---
                if let Some(meter) = &mut lufs_meter {
                    if meter.add_frames_f32(&temp_buffer).is_ok() {
                        if let Ok(m) = meter.loudness_momentary() {
                            GLOBAL_STATE.lufs_master[0].store(f32::to_bits(m as f32), Ordering::Relaxed);
                        }
                        if let Ok(s) = meter.loudness_shortterm() {
                            GLOBAL_STATE.lufs_master[1].store(f32::to_bits(s as f32), Ordering::Relaxed);
                        }
                        if let Ok(i) = meter.loudness_global() {
                            GLOBAL_STATE.lufs_master[2].store(f32::to_bits(i as f32), Ordering::Relaxed);
                        }
                        let mut true_peak = 0.0;
                        for ch in 0..channels.min(meter.channels() as usize) {
                            if let Ok(p) = meter.true_peak(ch as u32) {
                                if p > true_peak {
                                    true_peak = p;
                                }
                            }
                        }
                        GLOBAL_STATE.lufs_master[3].store(f32::to_bits(true_peak as f32), Ordering::Relaxed);
                    }
                }
            } else {
                // usable > 0 인데 읽기에 실패하는 경우(경쟁 상태 등)는 바쁜
                // 재시도로 번지지 않도록 여기서도 쉰다.
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 10초 1kHz 사인(-20dBFS 근처) 스테레오를 측정기에 넣고 통합 음량을 돌려준다.
    fn integrated_of_sine(meter: &mut ebur128::EbuR128) -> f64 {
        let rate = 48_000;
        let mut buf = Vec::with_capacity(rate * 10 * 2);
        for n in 0..rate * 10 {
            let s = 0.1 * (2.0 * std::f32::consts::PI * 1000.0 * n as f32 / rate as f32).sin();
            buf.push(s);
            buf.push(s);
        }
        meter.add_frames_f32(&buf).expect("add_frames_f32");
        meter.loudness_global().expect("loudness_global")
    }

    #[test]
    fn integrated_loudness_uses_fixed_size_histogram() {
        // 기본(큐) 방식은 켠 뒤의 0.4초 블록을 모두 쌓고(초당 10개) 통합 음량을 물을 때마다 전부 다시
        // 훑는다. 이 스레드는 오디오가 들어올 때마다 묻기 때문에 무인 운영에서 메모리와 CPU가 시간에
        // 비례해 늘었다(2026-11-01 28시간 시험). 히스토그램은 칸 1000개로 고정이다.
        let meter = new_lufs_meter(2, 48_000).expect("측정기를 만들지 못했다");
        assert!(meter.mode().contains(ebur128::Mode::I));
        assert!(meter.mode().contains(ebur128::Mode::HISTOGRAM), "통합 음량이 큐 방식이다");
    }

    #[test]
    fn histogram_integrated_loudness_matches_queue_within_a_tenth_lu() {
        // 히스토그램은 0.1LU 칸으로 재므로 값이 거의 같아야 한다.
        let mut ours = new_lufs_meter(2, 48_000).expect("측정기를 만들지 못했다");
        let mut queue = ebur128::EbuR128::new(
            2,
            48_000,
            ebur128::Mode::M | ebur128::Mode::S | ebur128::Mode::I | ebur128::Mode::TRUE_PEAK,
        )
        .expect("큐 측정기");
        let (a, b) = (integrated_of_sine(&mut ours), integrated_of_sine(&mut queue));
        assert!((a - b).abs() < 0.1, "히스토그램 {a:.3} LUFS, 큐 {b:.3} LUFS");
    }
}
