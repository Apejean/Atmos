use rtrb::Consumer;
use std::sync::atomic::Ordering;
use crate::core::state::GLOBAL_STATE;
use crate::audio::rta::RtaAnalyzer;

pub fn start_analysis_thread(mut rx: Consumer<f32>, sample_rate: u32, channels: usize) {
    std::thread::spawn(move || {
        let mut rta_analyzer = RtaAnalyzer::new();
        {
            let mut lock = GLOBAL_STATE.rta_magnitudes_ref.write().unwrap();
            *lock = Some(rta_analyzer.get_magnitudes_arc());
        }

        let mut lufs_meter = ebur128::EbuR128::new(
            channels.max(1) as u32,
            sample_rate,
            ebur128::Mode::M | ebur128::Mode::S | ebur128::Mode::I | ebur128::Mode::TRUE_PEAK
        ).ok();

        let mut temp_buffer = Vec::with_capacity(16384);
        let mut mono_mix = Vec::with_capacity(16384);

        loop {
            if let Ok(chunk) = rx.read_chunk(rx.slots()) {
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
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
    });
}
