use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use rtrb::{Consumer, RingBuffer};
use rubato::{Resampler, SincFixedIn, InterpolationType, InterpolationParameters, WindowFunction};

#[repr(align(128))]
pub struct CachePadded<T> {
    pub value: T,
}

pub struct DiskStreamer {
    pub chunk_receiver: Option<Consumer<Vec<f32>>>,
    pub is_running: Arc<CachePadded<AtomicBool>>,
    pub sample_rate: u32,
    pub channels: u16,
    pub thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl Default for DiskStreamer {
    fn default() -> Self {
        let (_, rx) = RingBuffer::new(1);
        Self {
            chunk_receiver: Some(rx),
            is_running: Arc::new(CachePadded { value: AtomicBool::new(false) }),
            sample_rate: 48000,
            channels: 2,
            thread_handle: None,
        }
    }
}

impl Drop for DiskStreamer {
    fn drop(&mut self) {
        self.is_running.value.store(false, Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

impl DiskStreamer {
    pub fn new(file_path: String, is_loop: bool, target_sample_rate: u32) -> anyhow::Result<Self> {
        let (mut tx, rx) = RingBuffer::new(128); // Buffer up to 128 chunks
        let is_running = Arc::new(CachePadded { value: AtomicBool::new(true) });

        let path = std::path::PathBuf::from(file_path);
        let run_flag = is_running.clone();

        let file_for_probe = std::fs::File::open(&path)?;
        let mss_probe = symphonia::core::io::MediaSourceStream::new(
            Box::new(file_for_probe),
            Default::default(),
        );
        let mut hint = symphonia::core::probe::Hint::new();
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            hint.with_extension(&ext.to_lowercase());
        }
        let format_opts = symphonia::core::formats::FormatOptions::default();
        let metadata_opts = symphonia::core::meta::MetadataOptions::default();
        let decoder_opts = symphonia::core::codecs::DecoderOptions::default();

        let probed = symphonia::default::get_probe().format(
            &hint,
            mss_probe,
            &format_opts,
            &metadata_opts,
        )?;

        let track = probed
            .format
            .default_track()
            .ok_or_else(|| anyhow::anyhow!("No default track"))?;
        let src_sample_rate = track.codec_params.sample_rate.unwrap_or(48000);
        let channels = track
            .codec_params
            .channels
            .map(|c| c.count() as u16)
            .unwrap_or(2);
        let max_frames = track.codec_params.max_frames_per_packet.unwrap_or(4096) as usize;
        let track_id = track.id;
        
        let needs_resampling = src_sample_rate != target_sample_rate;
        let output_sample_rate = target_sample_rate;

        let handle = std::thread::spawn(move || {
            let file = match std::fs::File::open(&path) {
                Ok(f) => Box::new(f),
                Err(e) => {
                    eprintln!("DiskStreamer failed to open file: {}", e);
                    return;
                }
            };

            let mss = symphonia::core::io::MediaSourceStream::new(file, Default::default());
            let probed = match symphonia::default::get_probe().format(
                &hint,
                mss,
                &format_opts,
                &metadata_opts,
            ) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("DiskStreamer probe error: {}", e);
                    return;
                }
            };

            let mut format = probed.format;
            let track = match format.default_track() {
                Some(t) => t,
                None => {
                    eprintln!("DiskStreamer: No default track");
                    return;
                }
            };

            let mut decoder =
                match symphonia::default::get_codecs().make(&track.codec_params, &decoder_opts) {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("DiskStreamer decoder error: {}", e);
                        return;
                    }
                };

            let mut sample_buf = None;
            
            let mut resampler = if needs_resampling {
                let params = InterpolationParameters {
                    sinc_len: 256,
                    f_cutoff: 0.95,
                    interpolation: InterpolationType::Linear,
                    oversampling_factor: 256,
                    window: WindowFunction::BlackmanHarris2,
                };
                Some(SincFixedIn::<f32>::new(
                    target_sample_rate as f64 / src_sample_rate as f64,
                    2.0,
                    params,
                    max_frames,
                    channels as usize,
                ).unwrap())
            } else {
                None
            };
            
            let mut resampler_in_buf = vec![vec![0.0; max_frames]; channels as usize];

            while run_flag.value.load(Ordering::Relaxed) {
                let packet = match format.next_packet() {
                    Ok(p) => p,
                    Err(symphonia::core::errors::Error::ResetRequired) => {
                        decoder.reset();
                        continue;
                    }
                    Err(symphonia::core::errors::Error::IoError(err)) => {
                        if err.kind() == std::io::ErrorKind::UnexpectedEof {
                            if is_loop {
                                if let Ok(f) = std::fs::File::open(&path) {
                                    let mss_loop = symphonia::core::io::MediaSourceStream::new(
                                        Box::new(f),
                                        Default::default(),
                                    );
                                    if let Ok(probed_loop) = symphonia::default::get_probe().format(
                                        &hint,
                                        mss_loop,
                                        &format_opts,
                                        &metadata_opts,
                                    ) {
                                        format = probed_loop.format;
                                        decoder.reset();
                                        continue;
                                    }
                                }
                            } else {
                                break;
                            }
                        }
                        break;
                    }
                    Err(e) => {
                        eprintln!("Packet error: {}", e);
                        continue;
                    }
                };

                if packet.track_id() != track_id {
                    continue;
                }

                match decoder.decode(&packet) {
                    Ok(audio_buf) => {
                        if sample_buf.is_none() {
                            let spec = *audio_buf.spec();
                            let duration = std::cmp::max(audio_buf.capacity() as u64, max_frames as u64);
                            sample_buf = Some(symphonia::core::audio::SampleBuffer::<f32>::new(
                                duration, spec,
                            ));
                        }

                        if let Some(buf) = &mut sample_buf {
                            buf.copy_interleaved_ref(audio_buf);
                            
                            let mut chunk = Vec::new();
                            let frames = buf.samples().len() / channels as usize;
                            
                            if let Some(r) = &mut resampler {
                                // De-interleave
                                for ch in 0..channels as usize {
                                    resampler_in_buf[ch].clear();
                                }
                                for frame in 0..frames {
                                    for ch in 0..channels as usize {
                                        resampler_in_buf[ch].push(buf.samples()[frame * channels as usize + ch]);
                                    }
                                }
                                
                                // Resample
                                match r.process(&resampler_in_buf, None) {
                                    Ok(resampled) => {
                                        let out_frames = resampled[0].len();
                                        chunk.reserve(out_frames * channels as usize);
                                        // Interleave
                                        for frame in 0..out_frames {
                                            for ch in 0..channels as usize {
                                                chunk.push(resampled[ch][frame]);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Resampling error: {}", e);
                                        continue;
                                    }
                                }
                            } else {
                                chunk.extend_from_slice(buf.samples());
                            }

                            let mut item = chunk;
                            loop {
                                if !run_flag.value.load(Ordering::Relaxed) {
                                    break;
                                }
                                match tx.push(item) {
                                    Ok(_) => break,
                                    Err(rtrb::PushError::Full(returned)) => {
                                        std::thread::sleep(std::time::Duration::from_millis(1));
                                        item = returned;
                                    }
                                }
                            }
                        }
                    }
                    Err(symphonia::core::errors::Error::DecodeError(e)) => {
                        eprintln!("Decode error (ignoring): {}", e);
                    }
                    Err(e) => {
                        eprintln!("Fatal decode error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(Self {
            chunk_receiver: Some(rx),
            is_running,
            sample_rate: output_sample_rate,
            channels,
            thread_handle: Some(handle),
        })
    }
}
