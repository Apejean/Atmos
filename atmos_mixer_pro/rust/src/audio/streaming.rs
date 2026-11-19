use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use rtrb::{Consumer, RingBuffer};
use rubato::{Resampler, SincFixedOut, InterpolationType, InterpolationParameters, WindowFunction};

/// 다시 열기에 실패한 뒤 `attempt`번째 시도까지 기다리는 시간: 1·2·5초, 그다음부터 10초마다(검토 A6, 엔진
/// 유지자·OSC 포트 다시 열기와 같은 방향). 포기하지 않는다.
pub fn reopen_delay(attempt: u32) -> std::time::Duration {
    std::time::Duration::from_secs(match attempt {
        0 | 1 => 1,
        2 => 2,
        3 => 5,
        _ => 10,
    })
}

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
        
        let needs_resampling = src_sample_rate != target_sample_rate;
        let output_sample_rate = if needs_resampling { target_sample_rate } else { src_sample_rate };
        // 이 위치까지는 디코딩만 하고 버린다. 믹서가 세는 위치와 같게 출력(리샘플링 뒤) 프레임으로 센다.
        let mut skip_samples =
            (start_seconds.max(0.0) * output_sample_rate as f64).round() as usize * channels as usize;
        let total_frames = track.codec_params.n_frames;
        let loop_len_frames = total_frames.map(|n| n as f64 * output_sample_rate as f64 / src_sample_rate as f64);
        let first_chunk_ready = Arc::new(AtomicBool::new(false));
        let ready_flag = first_chunk_ready.clone();

        let ctx = OpenCtx {
            path,
            hint,
            format_opts,
            metadata_opts,
            decoder_opts,
            total_frames,
            src_rate: src_sample_rate,
            unknown_length_give_up: UNKNOWN_LENGTH_GIVE_UP,
        };
        let handle = std::thread::spawn(move || {
            // 처음 열기. 실패하면 사건 로그만 남기고 끝낸다(검토 A6 (c) — new_at이 바로 앞에서 한 번 열어 봤으므로 드물다).
            let mut src = match open_source(&ctx) {
                Ok(s) => s,
                Err(e) => {
                    crate::core::event_log::log_event(
                        "stream_open_failed",
                        &[("path", &ctx.path.to_string_lossy()), ("error", &e.to_string())],
                    );
                    return;
                }
            };

            // 시작 위치로 바로 찾는다(그룹 시작 설계 3.5). 처음부터 해독해 버리면 위치가 뒤일수록 첫 묶음이 늦고 그
            // 시간이 트랙마다 달라진다(진단 T264: 290초 이어 가기에서 최대 103ms 어긋남). 샘플레이트를 바꾸지 않는 파일만
            // 찾고(출력 프레임 = 파일 프레임), 찾은 자리와 요청한 자리의 차이만 해독해 버려 샘플 단위로 맞춘다. 찾기를
            // 못 하는 형식이면 예전처럼 처음부터 해독해 버린다.
            // 마지막으로 해독한 묶음의 다음 소스 프레임. 읽기 오류 뒤 다시 열면 여기부터 잇는다(끊긴 자리).
            let mut next_ts: u64 = 0;
            if skip_samples > 0 && !needs_resampling {
                let ch = (channels as usize).max(1);
                let mut target = (skip_samples / ch) as u64;
                if let Some(len) = total_frames {
                    if is_loop && len > 0 {
                        target %= len;
                    }
                }
                let to = symphonia::core::formats::SeekTo::TimeStamp { ts: target, track_id: src.track_id };
                if let Ok(seeked) = src.format.seek(symphonia::core::formats::SeekMode::Accurate, to) {
                    src.decoder.reset();
                    skip_samples = target.saturating_sub(seeked.actual_ts) as usize * ch;
                    next_ts = seeked.actual_ts;
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

            // 다시 연 뒤 이 소스 프레임 앞은 해독해 버린다(찾기가 그 앞 묶음에 서거나 찾기를 못 하는 형식).
            let mut resume_from: Option<u64> = None;
            let mut consecutive_errors: u32 = 0;
            let mut recovery = Recovery::default();

            while run_flag.value.load(Ordering::Relaxed) {
                let packet = match src.format.next_packet() {
                    Ok(p) => p,
                    Err(symphonia::core::errors::Error::ResetRequired) => {
                        src.decoder.reset();
                        continue;
                    }
                    Err(symphonia::core::errors::Error::IoError(err))
                        if err.kind() == std::io::ErrorKind::UnexpectedEof =>
                    {
                        if !is_loop {
                            break;
                        }
                        // 루프 한 바퀴 끝: 다음 바퀴를 처음부터 연다. 못 열면 포기하지 않고 기다렸다 다시 연다.
                        match reopen(&ctx, 0, ReopenCause::LoopEnd, None, is_loop, &mut recovery, &run_flag) {
                            Some(s) => {
                                src = s;
                                next_ts = 0;
                                resume_from = None;
                                consecutive_errors = 0;
                                continue;
                            }
                            None => break,
                        }
                    }
                    Err(symphonia::core::errors::Error::IoError(err)) => {
                        // 파일 끝이 아닌 읽기 오류(디스크·USB·네트워크): 루프·단발 모두 끊긴 자리부터 다시 연다(A6 (a)).
                        let detail = err.to_string();
                        match reopen(&ctx, next_ts, ReopenCause::ReadError, Some(&detail), is_loop, &mut recovery, &run_flag) {
                            Some(s) => {
                                src = s;
                                resume_from = resume_point(resume_from, next_ts);
                                consecutive_errors = 0;
                                continue;
                            }
                            None => break,
                        }
                    }
                    Err(e) => {
                        // 같은 오류가 계속되면 바쁜 반복이 된다. 연속 상한을 넘으면 읽기 오류처럼 다시 연다(A6 (b)).
                        consecutive_errors += 1;
                        if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                            let detail = e.to_string();
                            match reopen(&ctx, next_ts, ReopenCause::RepeatedErrors, Some(&detail), is_loop, &mut recovery, &run_flag) {
                                Some(s) => {
                                    src = s;
                                    resume_from = resume_point(resume_from, next_ts);
                                    consecutive_errors = 0;
                                }
                                None => break,
                            }
                        }
                        continue;
                    }
                };

                if packet.track_id() != src.track_id {
                    continue;
                }
                let packet_ts = packet.ts();
                let packet_end = packet_ts + packet.dur();

                match src.decoder.decode(&packet) {
                    Ok(audio_buf) => {
                        consecutive_errors = 0;
                        next_ts = packet_end;
                        if sample_buf.is_none() {
                            let spec = *audio_buf.spec();
                            sample_buf = Some(symphonia::core::audio::SampleBuffer::<f32>::new(
                                audio_buf.capacity() as u64, spec,
                            ));
                        }

                        if let Some(buf) = &mut sample_buf {
                            buf.copy_interleaved_ref(audio_buf);
                            let ch_count = (channels as usize).max(1);
                            // 다시 연 자리 앞은 버린다(끊긴 자리부터 정확히 잇는다).
                            let mut drop_frames = 0usize;
                            if let Some(from) = resume_from {
                                if packet_end <= from {
                                    continue;
                                }
                                drop_frames = from.saturating_sub(packet_ts) as usize;
                                resume_from = None;
                            }
                            let all = buf.samples();
                            let samples = &all[(drop_frames * ch_count).min(all.len())..];
                            // 소리가 다시 나간다: 기다리던 중이었으면 회복으로 적고 실패 목록에서 뺀다.
                            recovery.progressed(&ctx, is_loop);

                            if let Some(r) = &mut resampler {
                                let frames = samples.len() / ch_count;
                                for ch in 0..channels as usize {
                                    for frame in 0..frames {
                                        input_accumulator[ch].push(samples[frame * ch_count + ch]);
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
                            } else if !samples.is_empty() {
                                let mut chunk = Vec::with_capacity(samples.len());
                                chunk.extend_from_slice(samples);
                                send_chunk(&mut tx, &run_flag, &ready_flag, &mut skip_samples, chunk);
                            }
                        }
                    }
                    Err(e) => {
                        // 깨진 묶음은 건너뛴다(다시 열면 그 뒤부터). 연속 상한을 넘으면 다시 연다(A6 (b)).
                        next_ts = packet_end;
                        consecutive_errors += 1;
                        if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                            let detail = e.to_string();
                            match reopen(&ctx, next_ts, ReopenCause::RepeatedErrors, Some(&detail), is_loop, &mut recovery, &run_flag) {
                                Some(s) => {
                                    src = s;
                                    resume_from = resume_point(resume_from, next_ts);
                                    consecutive_errors = 0;
                                }
                                None => break,
                            }
                        }
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
            loop_len_frames,
            source_sample_rate: src_sample_rate,
            first_chunk_ready,
        })
    }
}

/// 다시 열기를 기다리는 동안 정지(실행 깃발)를 보는 간격. 정지에 100ms 안에 반응한다.
const WAIT_SLICE: std::time::Duration = std::time::Duration::from_millis(50);
/// 길이를 모르는 단발(예: Xing/Info 머리말 없는 MP3)은 다시 열기를 이만큼 기다린 뒤 포기한다. 끝없이 기다리면
/// 재생 중으로 남아 모든 루프를 더킹시킨다(검토 R1 보완).
pub const UNKNOWN_LENGTH_GIVE_UP: std::time::Duration = std::time::Duration::from_secs(60);
/// 읽기·해독 오류가 연속으로 이만큼 나면 읽기 오류처럼 다시 연다(같은 오류의 바쁜 반복 막기, 검토 A6 (b)).
pub const MAX_CONSECUTIVE_ERRORS: u32 = 100;

/// 디코더 스레드가 파일을 (다시) 여는 데 필요한 것.
struct OpenCtx {
    path: std::path::PathBuf,
    hint: symphonia::core::probe::Hint,
    format_opts: symphonia::core::formats::FormatOptions,
    metadata_opts: symphonia::core::meta::MetadataOptions,
    decoder_opts: symphonia::core::codecs::DecoderOptions,
    /// 파일 길이(소스 프레임). 모르면 None. 단발이 언제 포기할지 정한다.
    total_frames: Option<u64>,
    /// 소스 샘플레이트.
    src_rate: u32,
    /// 길이를 모르는 단발이 포기하기까지 기다리는 시간(운영은 [`UNKNOWN_LENGTH_GIVE_UP`], 시험은 줄인다).
    unknown_length_give_up: std::time::Duration,
}

/// 열린 파일: 읽개·디코더·트랙 번호.
struct Source {
    format: Box<dyn symphonia::core::formats::FormatReader>,
    decoder: Box<dyn symphonia::core::codecs::Decoder>,
    track_id: u32,
}

fn open_source(ctx: &OpenCtx) -> anyhow::Result<Source> {
    let file = std::fs::File::open(&ctx.path)?;
    let mss = symphonia::core::io::MediaSourceStream::new(Box::new(file), Default::default());
    let probed = symphonia::default::get_probe().format(&ctx.hint, mss, &ctx.format_opts, &ctx.metadata_opts)?;
    let format = probed.format;
    let track = format.default_track().ok_or_else(|| anyhow::anyhow!("No default track"))?;
    let track_id = track.id;
    let decoder = symphonia::default::get_codecs().make(&track.codec_params, &ctx.decoder_opts)?;
    Ok(Source { format, decoder, track_id })
}

/// `ts`(소스 프레임) 자리로 연다. 찾기(seek)가 되면 true. 안 되면 처음부터 다시 연 것을 돌려주고(부르는 쪽이 그
/// 자리까지 해독해 버린다) false.
fn open_source_at(ctx: &OpenCtx, ts: u64) -> anyhow::Result<(Source, bool)> {
    let mut src = open_source(ctx)?;
    if ts == 0 {
        return Ok((src, true));
    }
    let to = symphonia::core::formats::SeekTo::TimeStamp { ts, track_id: src.track_id };
    match src.format.seek(symphonia::core::formats::SeekMode::Accurate, to) {
        Ok(_) => {
            src.decoder.reset();
            Ok((src, true))
        }
        // 찾기에 실패한 읽개는 상태를 믿을 수 없어 새로 연다.
        Err(_) => Ok((open_source(ctx)?, false)),
    }
}

/// 왜 다시 여는가(사건 로그 `cause`).
#[derive(Clone, Copy, PartialEq)]
enum ReopenCause {
    /// 루프 한 바퀴 끝(평소).
    LoopEnd,
    /// 파일 끝이 아닌 읽기 오류.
    ReadError,
    /// 읽기·해독 오류가 연속 [`MAX_CONSECUTIVE_ERRORS`]번.
    RepeatedErrors,
}

impl ReopenCause {
    fn name(self) -> &'static str {
        match self {
            ReopenCause::LoopEnd => "loop_end",
            ReopenCause::ReadError => "read_error",
            ReopenCause::RepeatedErrors => "repeated_errors",
        }
    }
}

/// 다시 열기 실패 상태(디코더 스레드 하나). 실패 중이면 실패 목록 토큰을 쥐고 있어 배너가 보인다.
#[derive(Default)]
struct Recovery {
    attempt: u32,
    token: Option<crate::core::read_failures::FailureToken>,
    since: Option<std::time::Instant>,
    /// 오류로 다시 연 자리. 소리가 나기 전에 같은 자리에서 또 오류로 다시 열면 실패로 센다.
    error_at: Option<u64>,
    /// 단발이 남은 길이만큼 기다려 포기했다(검토 R1).
    gave_up: bool,
}

impl Recovery {
    /// 실패 한 번: 실패 목록에 넣고(처음이면) 사건 로그를 남긴 뒤(같은 파일 1분에 한 번) 다음 시도까지 기다린다.
    /// 기다리는 중에 정지되면 false. 단발은 처음 실패부터 기다린 시간이 `at_ts` 뒤 남은 길이를 넘으면 포기하고
    /// false(`gave_up`) — 그때면 원래 끝났을 시간이다(공연 시간 기준, 재시작의 restart_resume::skip_outage와 같은
    /// 방향, 검토 R1). 계속 기다리면 재생 중으로 남아 루프를 더킹시키고 몇 시간 뒤 남은 부분이 난다. 길이를 모르는
    /// 단발은 [`UNKNOWN_LENGTH_GIVE_UP`] 뒤 포기한다. 루프는 포기하지 않는다.
    fn failed(
        &mut self,
        ctx: &OpenCtx,
        cause: ReopenCause,
        error: &str,
        at_ts: u64,
        is_loop: bool,
        run_flag: &CachePadded<AtomicBool>,
    ) -> bool {
        self.attempt += 1;
        if self.token.is_none() {
            self.token = Some(crate::core::read_failures::begin(&ctx.path));
            self.since = Some(std::time::Instant::now());
        }
        if crate::core::read_failures::should_log_failure(&ctx.path) {
            let attempt = self.attempt.to_string();
            crate::core::event_log::log_event(
                "loop_reopen_failed",
                &[
                    ("path", &ctx.path.to_string_lossy()),
                    ("error", error),
                    ("attempt", &attempt),
                    ("cause", cause.name()),
                    ("loop", if is_loop { "true" } else { "false" }),
                ],
            );
        }
        let mut wait = reopen_delay(self.attempt);
        if !is_loop {
            // 단발이 기다릴 수 있는 시간: 남은 길이(원래 끝났을 때). 길이를 모르면 고정 상한.
            let (limit, remaining_s) = match ctx.total_frames {
                Some(total) => {
                    let remaining = std::time::Duration::from_secs_f64(
                        total.saturating_sub(at_ts) as f64 / ctx.src_rate.max(1) as f64,
                    );
                    (remaining, format!("{:.1}", remaining.as_secs_f64()))
                }
                None => (ctx.unknown_length_give_up, "unknown".to_string()),
            };
            let waited = self.since.map(|s| s.elapsed()).unwrap_or_default();
            if waited >= limit {
                let waited_s = format!("{:.1}", waited.as_secs_f64());
                crate::core::event_log::log_event(
                    "stream_reopen_gave_up",
                    &[
                        ("path", &ctx.path.to_string_lossy()),
                        ("waited_s", &waited_s),
                        ("remaining_s", &remaining_s),
                    ],
                );
                self.gave_up = true;
                return false;
            }
            // 기다릴 수 있는 때를 넘겨 기다리지 않는다: 그때 한 번 더 열어 보고 안 되면 포기한다.
            wait = wait.min(limit - waited);
        }
        wait_while_running(wait, run_flag)
    }

    /// 소리가 다시 나간다. 기다리던 중이었으면 회복을 남기고 실패 목록에서 뺀다.
    fn progressed(&mut self, ctx: &OpenCtx, is_loop: bool) {
        self.error_at = None;
        if let Some(since) = self.since.take() {
            let after = format!("{:.1}", since.elapsed().as_secs_f64());
            let attempts = self.attempt.to_string();
            crate::core::event_log::log_event(
                "loop_reopen_recovered",
                &[
                    ("path", &ctx.path.to_string_lossy()),
                    ("after_s", &after),
                    ("attempts", &attempts),
                    ("loop", if is_loop { "true" } else { "false" }),
                ],
            );
            self.token = None;
            self.attempt = 0;
        }
    }
}

/// `d` 동안 기다린다. 실행 깃발이 내려가면 곧바로 false.
fn wait_while_running(d: std::time::Duration, run_flag: &CachePadded<AtomicBool>) -> bool {
    let deadline = std::time::Instant::now() + d;
    loop {
        if !run_flag.value.load(Ordering::Relaxed) {
            return false;
        }
        let now = std::time::Instant::now();
        if now >= deadline {
            return true;
        }
        std::thread::sleep(WAIT_SLICE.min(deadline - now));
    }
}

/// 파일을 `at_ts`(소스 프레임) 자리로 다시 연다. 못 열면 [`reopen_delay`] 간격으로 계속 시도한다(검토 A6). 정지되거나
/// 단발이 남은 길이만큼 기다려 포기하면 None(스레드가 끝나 믹서가 트랙을 정상 종료한다). 오류로 다시 연 자리에서 소리가 나기 전에 또 오류가 나면(열리기는 하는데 같은 자리를
/// 못 읽는 경우) 그것도 실패로 세어 기다린다 — 바쁜 반복을 막는다.
fn reopen(
    ctx: &OpenCtx,
    at_ts: u64,
    cause: ReopenCause,
    error: Option<&str>,
    is_loop: bool,
    rec: &mut Recovery,
    run_flag: &CachePadded<AtomicBool>,
) -> Option<Source> {
    if cause != ReopenCause::LoopEnd {
        if rec.error_at == Some(at_ts) {
            if !rec.failed(ctx, cause, error.unwrap_or(""), at_ts, is_loop, run_flag) {
                return None;
            }
        } else if rec.since.is_none() {
            let at = at_ts.to_string();
            crate::core::event_log::log_event(
                "stream_read_error",
                &[
                    ("path", &ctx.path.to_string_lossy()),
                    ("at_frame", &at),
                    ("cause", cause.name()),
                    ("error", error.unwrap_or("")),
                ],
            );
        }
        rec.error_at = Some(at_ts);
    }
    loop {
        if !run_flag.value.load(Ordering::Relaxed) {
            return None;
        }
        match open_source_at(ctx, at_ts) {
            Ok((src, seeked)) => {
                if !seeked {
                    let at = at_ts.to_string();
                    crate::core::event_log::log_event(
                        "stream_seek_unavailable",
                        &[("path", &ctx.path.to_string_lossy()), ("at_frame", &at)],
                    );
                }
                return Some(src);
            }
            Err(e) => {
                if !rec.failed(ctx, cause, &e.to_string(), at_ts, is_loop, run_flag) {
                    return None;
                }
            }
        }
    }
}

/// 오류로 다시 연 뒤 이어 보낼 소스 프레임. 찾기를 못 해 처음부터 해독해 버리는 중(`pending`)에 또 오류가 나면
/// `next_ts`는 버리던 묶음의 끝이라 원래 자리보다 앞이다. 그 앞 값으로 덮으면 이미 보낸 구간을 다시 보내 위상이
/// 밀리므로 더 뒤인 쪽을 지킨다.
fn resume_point(pending: Option<u64>, next_ts: u64) -> Option<u64> {
    Some(pending.map_or(next_ts, |from| from.max(next_ts)))
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

#[cfg(test)]
mod tests {
    use super::resume_point;

    use super::{reopen, CachePadded, OpenCtx, Recovery, ReopenCause};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    /// 실패 목록(전역)을 쓰는 시험은 하나씩 돈다.
    static SERIAL: Mutex<()> = Mutex::new(());

    fn missing_ctx(tag: &str, total_frames: Option<u64>) -> OpenCtx {
        OpenCtx {
            path: std::env::temp_dir().join(format!("atmos_missing_{tag}_{}.wav", std::process::id())),
            hint: Default::default(),
            format_opts: Default::default(),
            metadata_opts: Default::default(),
            decoder_opts: Default::default(),
            total_frames,
            src_rate: 48_000,
            unknown_length_give_up: Duration::from_millis(300),
        }
    }

    #[test]
    fn 단발은_남은_길이만큼_기다린_뒤_포기하고_실패_목록에서_빠진다() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        // 1초짜리 파일의 0.8초 자리에서 끊겼다: 남은 0.2초가 지나면 원래 끝났을 시간이라 포기한다(검토 R1).
        let ctx = missing_ctx("oneshot", Some(48_000));
        let run = CachePadded { value: AtomicBool::new(true) };
        let mut rec = Recovery::default();
        let t0 = Instant::now();
        let got = reopen(&ctx, 38_400, ReopenCause::ReadError, Some("test"), false, &mut rec, &run);
        let took = t0.elapsed();
        assert!(got.is_none());
        assert!(rec.gave_up, "정지가 아니라 포기로 끝나야 한다");
        assert!(took >= Duration::from_millis(150), "남은 길이보다 일찍 포기했다: {took:?}");
        assert!(took < Duration::from_millis(900), "남은 길이(0.2초)를 넘겨 기다렸다: {took:?}");
        assert_eq!(crate::core::read_failures::failing_count(), 1, "기다리는 동안은 배너에 있어야 한다");
        drop(rec); // 스레드가 끝나면 함께 버려진다
        assert_eq!(crate::core::read_failures::failing_count(), 0);
    }

    fn keeps_waiting(ctx: OpenCtx, is_loop: bool, cause: ReopenCause) {
        let run = Arc::new(CachePadded { value: AtomicBool::new(true) });
        let run2 = run.clone();
        let h = std::thread::spawn(move || {
            let mut rec = Recovery::default();
            let got = reopen(&ctx, 0, cause, None, is_loop, &mut rec, &run2);
            (got.is_none(), rec.gave_up)
        });
        std::thread::sleep(Duration::from_millis(1_500));
        assert!(!h.is_finished(), "포기하면 안 되는데 끝났다");
        run.value.store(false, Ordering::Relaxed);
        let t0 = Instant::now();
        let (none, gave_up) = h.join().unwrap_or((false, true));
        assert!(t0.elapsed() < Duration::from_millis(200), "정지에 늦게 반응했다");
        assert!(none && !gave_up, "정지로 끝나야 한다(포기 아님)");
    }

    #[test]
    fn 루프는_포기하지_않는다() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        keeps_waiting(missing_ctx("loop", Some(4_800)), true, ReopenCause::LoopEnd);
    }

    #[test]
    fn 길이를_모르는_단발은_정한_시간만큼_기다린_뒤_포기한다() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        // 운영 상한은 60초, 시험은 0.3초로 줄였다(검토 R1 보완).
        assert_eq!(super::UNKNOWN_LENGTH_GIVE_UP, Duration::from_secs(60));
        let ctx = missing_ctx("unknown", None);
        let run = CachePadded { value: AtomicBool::new(true) };
        let mut rec = Recovery::default();
        let t0 = Instant::now();
        let got = reopen(&ctx, 0, ReopenCause::ReadError, Some("test"), false, &mut rec, &run);
        let took = t0.elapsed();
        assert!(got.is_none() && rec.gave_up, "길이를 모르는 단발도 포기해야 한다");
        assert!(took >= Duration::from_millis(250) && took < Duration::from_millis(1_000), "{took:?}");
        drop(rec);
        assert_eq!(crate::core::read_failures::failing_count(), 0);
    }

    #[test]
    fn 버리는_중에_또_오류가_나도_이어_보낼_자리는_뒤로_가지_않는다() {
        assert_eq!(resume_point(None, 1_000), Some(1_000), "평소에는 끊긴 자리");
        assert_eq!(resume_point(Some(50_000), 1_152), Some(50_000), "버리던 중이면 원래 자리를 지킨다");
        assert_eq!(resume_point(Some(50_000), 60_000), Some(60_000), "이미 지났으면 끊긴 자리");
    }
}
