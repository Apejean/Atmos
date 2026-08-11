use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use rtrb::{Consumer, RingBuffer};

#[repr(align(128))]
pub struct CachePadded<T> {
    pub value: T,
}

pub struct DiskStreamer {
    pub chunk_receiver: Option<Arc<std::sync::Mutex<Option<Consumer<Vec<f32>>>>>>,
    pub is_running: Arc<CachePadded<AtomicBool>>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl Default for DiskStreamer {
    fn default() -> Self {
        let (_, rx) = RingBuffer::new(1);
        Self {
            chunk_receiver: Some(Arc::new(std::sync::Mutex::new(Some(rx)))),
            is_running: Arc::new(CachePadded { value: AtomicBool::new(false) }),
            sample_rate: 48000,
            channels: 2,
        }
    }
}

impl DiskStreamer {
    pub fn new(file_path: String, is_loop: bool) -> anyhow::Result<Self> {
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
        let sample_rate = track.codec_params.sample_rate.unwrap_or(48000);
        let channels = track
            .codec_params
            .channels
            .map(|c| c.count() as u16)
            .unwrap_or(2);
        let max_frames = track.codec_params.max_frames_per_packet.unwrap_or(4096);
        let track_id = track.id;

        std::thread::spawn(move || {
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
                                // Re-open and reset for loop
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
                                break; // EOF reached, stop stream naturally for SFX
                            }
                        }
                        break; // Stop on severe IO error
                    }
                    Err(e) => {
                        eprintln!("Packet error: {}", e);
                        // For bad metadata/tags interpreted as packets, we should just continue instead of breaking
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
                            // Update actual channels dynamically (note: consumer might need to adapt if it reads `channels` field before first chunk, but mixer relies on chunk size if possible)
                            let duration = std::cmp::max(audio_buf.capacity() as u64, max_frames);
                            sample_buf = Some(symphonia::core::audio::SampleBuffer::<f32>::new(
                                duration, spec,
                            ));
                        }

                        if let Some(buf) = &mut sample_buf {
                            buf.copy_interleaved_ref(audio_buf);
                            // Batch into chunks
                            let mut chunk = Vec::with_capacity(buf.samples().len());
                            chunk.extend_from_slice(buf.samples());

                            // Send chunk to audio thread
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
            chunk_receiver: Some(Arc::new(std::sync::Mutex::new(Some(rx)))),
            is_running,
            sample_rate,
            channels,
        })
    }
}
