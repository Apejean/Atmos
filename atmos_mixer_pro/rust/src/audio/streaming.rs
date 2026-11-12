use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use rtrb::{Consumer, RingBuffer};
use rubato::{Resampler, SincFixedOut, InterpolationType, InterpolationParameters, WindowFunction};

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
    /// 루프 한 바퀴 길이(출력 프레임). 모르면 None.
    pub loop_len_frames: Option<f64>,
    /// 바꾸기 전 음원의 원래 샘플레이트. [Self::sample_rate]와 다르면 HQ sinc로 바꾸고 있다(화면 배지).
    pub source_sample_rate: u32,
    /// 첫 묶음을 링에 넣었다. 그룹 시작이 모든 트랙의 첫 묶음을 기다린 뒤 함께 시작하는 데 쓴다(그룹 시작 설계 3.1).
    pub first_chunk_ready: Arc<AtomicBool>,
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
            loop_len_frames: None,
            source_sample_rate: 48000,
            first_chunk_ready: Arc::new(AtomicBool::new(true)),
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
        Self::new_at(file_path, is_loop, target_sample_rate, 0.0)
    }

    /// `start_seconds`(파일 기준)부터 내보낸다. 재시작 복원이 멈춘 위치를 넘긴다.
    pub fn new_at(
        file_path: String,
        is_loop: bool,
        target_sample_rate: u32,
        start_seconds: f64,
    ) -> anyhow::Result<Self> {
        let (mut tx, rx) = RingBuffer::new(128);
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

        let probed = symphonia::default::get_probe().format(&hint, mss_probe, &format_opts, &metadata_opts)?;

        let track = probed.format.default_track().ok_or_else(|| anyhow::anyhow!("No default track"))?;
        let src_sample_rate = track.codec_params.sample_rate.unwrap_or(48000);
        let channels = track.codec_params.channels.map(|c| c.count() as u16).unwrap_or(2);
        let track_id = track.id;
        
        let needs_resampling = src_sample_rate != target_sample_rate;
        let output_sample_rate = if needs_resampling { target_sample_rate } else { src_sample_rate };
        // 이 위치까지는 디코딩만 하고 버린다. 믹서가 세는 위치와 같게 출력(리샘플링 뒤) 프레임으로 센다.
        let mut skip_samples =
            (start_seconds.max(0.0) * output_sample_rate as f64).round() as usize * channels as usize;
        let total_frames = track.codec_params.n_frames;
        let loop_len_frames = total_frames.map(|n| n as f64 * output_sample_rate as f64 / src_sample_rate as f64);
        let first_chunk_ready = Arc::new(AtomicBool::new(false));
        let ready_flag = first_chunk_ready.clone();

        let handle = std::thread::spawn(move || {
            let file = match std::fs::File::open(&path) {
                Ok(f) => Box::new(f),
                Err(e) => {
                    eprintln!("DiskStreamer failed to open file: {}", e);
                    return;
                }
            };

            let mss = symphonia::core::io::MediaSourceStream::new(file, Default::default());
            let probed = match symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("DiskStreamer probe error: {}", e);
                    return;
                }
            };

            let mut format = probed.format;
            let track = match format.default_track() {
                Some(t) => t,
                None => return,
            };

            let mut decoder = match symphonia::default::get_codecs().make(&track.codec_params, &decoder_opts) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("DiskStreamer decoder error: {}", e);
                    return;
                }
            };

            // 시작 위치로 바로 찾는다(그룹 시작 설계 3.5). 처음부터 해독해 버리면 위치가 뒤일수록 첫 묶음이 늦고 그
            // 시간이 트랙마다 달라진다(진단 T264: 290초 이어 가기에서 최대 103ms 어긋남). 샘플레이트를 바꾸지 않는 파일만
            // 찾고(출력 프레임 = 파일 프레임), 찾은 자리와 요청한 자리의 차이만 해독해 버려 샘플 단위로 맞춘다. 찾기를
            // 못 하는 형식이면 예전처럼 처음부터 해독해 버린다.
            if skip_samples > 0 && !needs_resampling {
                let ch = (channels as usize).max(1);
                let mut target = (skip_samples / ch) as u64;
                if let Some(len) = total_frames {
                    if is_loop && len > 0 {
                        target %= len;
                    }
                }
                let to = symphonia::core::formats::SeekTo::TimeStamp { ts: target, track_id };
                if let Ok(seeked) = format.seek(symphonia::core::formats::SeekMode::Accurate, to) {
                    decoder.reset();
                    skip_samples = target.saturating_sub(seeked.actual_ts) as usize * ch;
                }
            }

            let mut sample_buf = None;
            
            let mut resampler = if needs_resampling {
                let params = InterpolationParameters {
                    sinc_len: 256,
                    f_cutoff: 0.95,
                    interpolation: InterpolationType::Linear,
                    oversampling_factor: 256,
                    window: WindowFunction::BlackmanHarris2,
                };
                // Fixed output chunk size of 1024 frames
                Some(SincFixedOut::<f32>::new(
                    target_sample_rate as f64 / src_sample_rate as f64,
                    2.0,
                    params,
                    1024,
                    channels as usize,
                ).expect("Failed to create resampler"))
            } else {
                None
            };
            
            // Buffer to accumulate input samples for SincFixedOut
            let mut input_accumulator = vec![Vec::<f32>::new(); channels as usize];

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
                    Err(_) => continue,
                };

                if packet.track_id() != track_id {
                    continue;
                }

                match decoder.decode(&packet) {
                    Ok(audio_buf) => {
                        if sample_buf.is_none() {
                            let spec = *audio_buf.spec();
                            sample_buf = Some(symphonia::core::audio::SampleBuffer::<f32>::new(
                                audio_buf.capacity() as u64, spec,
                            ));
                        }

                        if let Some(buf) = &mut sample_buf {
                            buf.copy_interleaved_ref(audio_buf);
                            
                            if let Some(r) = &mut resampler {
                                let frames = buf.samples().len() / channels as usize;
                                for ch in 0..channels as usize {
                                    for frame in 0..frames {
                                        input_accumulator[ch].push(buf.samples()[frame * channels as usize + ch]);
                                    }
                                }
                                
                                while input_accumulator[0].len() >= r.input_frames_next() {
                                    let required = r.input_frames_next();
                                    let mut process_buf = vec![vec![0.0; required]; channels as usize];
                                    for ch in 0..channels as usize {
                                        let drained: Vec<f32> = input_accumulator[ch].drain(0..required).collect();
                                        process_buf[ch].copy_from_slice(&drained);
                                    }
                                    
                                    if let Ok(resampled) = r.process(&process_buf, None) {
                                        let out_frames = resampled[0].len();
                                        let mut chunk = Vec::with_capacity(out_frames * channels as usize);
                                        for frame in 0..out_frames {
                                            for ch in 0..channels as usize {
                                                chunk.push(resampled[ch][frame]);
                                            }
                                        }
                                        send_chunk(&mut tx, &run_flag, &ready_flag, &mut skip_samples, chunk);
                                    }
                                }
                            } else {
                                let mut chunk = Vec::with_capacity(buf.samples().len());
                                chunk.extend_from_slice(buf.samples());
                                send_chunk(&mut tx, &run_flag, &ready_flag, &mut skip_samples, chunk);
                            }
                        }
                    }
                    Err(_) => {}
                }
            }
        });

        Ok(Self {
            chunk_receiver: Some(rx),
            is_running,
            sample_rate: output_sample_rate,
            channels,
            thread_handle: Some(handle),
            loop_len_frames,
            source_sample_rate: src_sample_rate,
            first_chunk_ready,
        })
    }
}

/// 파일의 프레임 수와 샘플레이트(머리말만 읽는다). 모르면 None. 그룹 이어 가기가 길이가 같은 트랙을 가리는 데
/// 쓴다(core::group_start::align_plan). 오디오 스레드에서 부르지 않는다(파일 I/O).
pub fn file_frames(path: &str) -> Option<(u64, u32)> {
    let file = std::fs::File::open(path).ok()?;
    let mss = symphonia::core::io::MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = symphonia::core::probe::Hint::new();
    if let Some(ext) = std::path::Path::new(path).extension().and_then(|s| s.to_str()) {
        hint.with_extension(&ext.to_lowercase());
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &Default::default(), &Default::default())
        .ok()?;
    let track = probed.format.default_track()?;
    Some((track.codec_params.n_frames?, track.codec_params.sample_rate?))
}

/// 시작 위치까지의 샘플은 버리고 나머지를 링버퍼로 보낸다. 링이 차 있으면 자리가 날 때까지 기다린다(디코더 스레드).
/// 링에 넣으면 `ready`를 켠다(첫 묶음 준비, 그룹 시작이 기다린다).
fn send_chunk(
    tx: &mut rtrb::Producer<Vec<f32>>,
    run_flag: &CachePadded<AtomicBool>,
    ready: &AtomicBool,
    skip_samples: &mut usize,
    mut chunk: Vec<f32>,
) {
    if *skip_samples > 0 {
        if chunk.len() <= *skip_samples {
            *skip_samples -= chunk.len();
            return;
        }
        chunk.drain(..*skip_samples);
        *skip_samples = 0;
    }
    let mut item = chunk;
    loop {
        if !run_flag.value.load(Ordering::Relaxed) {
            break;
        }
        match tx.push(item) {
            Ok(_) => {
                ready.store(true, Ordering::Release);
                break;
            }
            Err(rtrb::PushError::Full(returned)) => {
                std::thread::sleep(std::time::Duration::from_millis(1));
                item = returned;
            }
        }
    }
}
