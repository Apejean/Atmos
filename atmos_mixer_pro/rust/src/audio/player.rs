use std::sync::Arc;

use rubato::{InterpolationParameters, InterpolationType, Resampler, SincFixedOut, WindowFunction};
use std::fs::File;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub struct SoundData {
    pub samples: Vec<f32>,
    pub channels: u16,
    pub sample_rate: u32,
}

pub fn downsample_hi_res_if_needed(
    in_sample_rate: u32,
    samples: Vec<f32>,
    channels: usize,
) -> (Vec<f32>, u32) {
    if in_sample_rate > 96000 {
        let target_rate = 48000;
        let step = (in_sample_rate as f32 / target_rate as f32) as usize;
        if step > 1 {
            let total_frames = samples.len() / channels;
            let out_frames = total_frames / step;
            let mut downsampled = Vec::with_capacity(out_frames * channels);

            for frame in 0..out_frames {
                let src_frame = frame * step;
                for ch in 0..channels {
                    let idx = src_frame * channels + ch;
                    if idx < samples.len() {
                        downsampled.push(samples[idx]);
                    } else {
                        downsampled.push(0.0);
                    }
                }
            }
            return (downsampled, target_rate);
        }
    }
    (samples, in_sample_rate)
}

impl SoundData {
    pub fn probe_channels(path: &std::path::Path) -> u32 {
        // 27채널 이상·RF64 WAV는 자체 리더의 머리말로 센다(안정성 ⑨). 예전에는 symphonia가 열지 못해 아래에서
        // 2(스테레오)를 돌려줘 화면과 처리 폭이 틀렸다.
        if let crate::audio::wav_reader::Choice::Own(info) = crate::audio::wav_reader::choose(path) {
            return info.channels as u32;
        }
        if let Ok(file) = File::open(path) {
            let mss = MediaSourceStream::new(Box::new(file), Default::default());
            let mut hint = Hint::new();
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                hint.with_extension(&ext.to_lowercase());
            }
            let format_opts = FormatOptions::default();
            let metadata_opts = MetadataOptions::default();
            if let Ok(probed) =
                symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts)
            {
                if let Some(track) = probed.format.default_track() {
                    return track
                        .codec_params
                        .channels
                        .map(|c| c.count() as u32)
                        .unwrap_or(2);
                }
            }
        }
        2
    }

    pub fn load_from_file(path: &std::path::Path, target_sample_rate: u32) -> anyhow::Result<Self> {
        Self::load_from_file_with_source(path, target_sample_rate).map(|(data, _)| data)
    }

    /// [Self::load_from_file]과 같고, 바꾸기 전 음원의 원래 샘플레이트를 함께 돌려준다(화면의 리샘플링 배지).
    pub fn load_from_file_with_source(
        path: &std::path::Path,
        target_sample_rate: u32,
    ) -> anyhow::Result<(Self, u32)> {
        // 27채널 이상·RF64 WAV는 자체 리더로 끝까지 읽는다(안정성 ⑨). 그 밖은 지금의 symphonia 길이다.
        let (mut all_samples, channels, src_sample_rate) = match crate::audio::wav_reader::choose(path) {
            crate::audio::wav_reader::Choice::Own(info) => Self::read_with_own_reader(path, info)?,
            crate::audio::wav_reader::Choice::Unsupported(msg) => return Err(anyhow::anyhow!(msg)),
            crate::audio::wav_reader::Choice::Symphonia => Self::decode_with_symphonia(path)?,
        };
        let actual_channels = channels;

        let needs_resampling = src_sample_rate != target_sample_rate && target_sample_rate > 0;

        if needs_resampling && !all_samples.is_empty() {
            let frames = all_samples.len() / channels as usize;
            let mut deinterleaved = vec![vec![0.0; frames]; channels as usize];
            for frame in 0..frames {
                for ch in 0..channels as usize {
                    deinterleaved[ch][frame] = all_samples[frame * channels as usize + ch];
                }
            }

            let params = InterpolationParameters {
                sinc_len: 256,
                f_cutoff: 0.95,
                interpolation: InterpolationType::Linear,
                oversampling_factor: 256,
                window: WindowFunction::BlackmanHarris2,
            };

            let ratio = target_sample_rate as f64 / src_sample_rate as f64;
            let mut resampler = SincFixedOut::<f32>::new(ratio, 2.0, params, 1024, channels as usize)
                .map_err(|e| anyhow::anyhow!("리샘플러를 만들지 못했다: {e}"))?;

            // 원래 길이 × 비율만큼 내보낸다. 입력이 끝나면 남은 자투리와 그 뒤를 0으로 채워 넣어, 리샘플러가
            // 앞을 내다보느라 붙잡고 있던 끝부분까지 다 나오게 한다. 예전에는 마지막 자투리를 버려 끝이 최대
            // 약 21ms(48kHz) 잘렸고, 변환 오류가 난 조각도 조용히 건너뛰었다(2026-10-24 기능 점검).
            let out_samples_total = (frames as f64 * ratio).round() as usize * channels as usize;
            let mut resampled_samples = Vec::with_capacity(out_samples_total + 1024 * channels as usize);
            let mut input_idx = 0;

            // Pre-allocate buffer outside the loop to avoid allocations
            let mut process_buf = vec![vec![0.0; 2048]; channels as usize];

            while resampled_samples.len() < out_samples_total {
                let required = resampler.input_frames_next();
                if required > process_buf[0].len() {
                    for ch in 0..channels as usize {
                        process_buf[ch].resize(required, 0.0);
                    }
                }

                let take = required.min(frames - input_idx);
                for ch in 0..channels as usize {
                    process_buf[ch][..take]
                        .copy_from_slice(&deinterleaved[ch][input_idx..input_idx + take]);
                    process_buf[ch][take..required].fill(0.0);
                }

                // Create a slice view of the exact required length
                let process_slice: Vec<&[f32]> =
                    process_buf.iter().map(|v| &v[..required]).collect();
                let resampled = resampler
                    .process(&process_slice, None)
                    .map_err(|e| anyhow::anyhow!("리샘플링 실패: {e}"))?;
                let out_frames = resampled[0].len();
                for frame in 0..out_frames {
                    for ch in 0..channels as usize {
                        resampled_samples.push(resampled[ch][frame]);
                    }
                }
                input_idx += take;
            }
            resampled_samples.truncate(out_samples_total);

            all_samples = resampled_samples;
        }

        Ok((
            Self {
                samples: all_samples,
                channels: actual_channels,
                sample_rate: if needs_resampling && target_sample_rate > 0 {
                    target_sample_rate
                } else {
                    src_sample_rate
                },
            },
            src_sample_rate,
        ))
    }

    /// symphonia로 끝까지 해독한다(26채널 이하 WAV와 그 밖의 형식). 인터리브 표본, 채널 수, 원래 샘플레이트.
    fn decode_with_symphonia(path: &std::path::Path) -> anyhow::Result<(Vec<f32>, u16, u32)> {
        let file = Box::new(File::open(path)?);
        let mss = MediaSourceStream::new(file, Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            hint.with_extension(&ext.to_lowercase());
        }
        let format_opts = FormatOptions::default();
        let metadata_opts = MetadataOptions::default();
        let decoder_opts = DecoderOptions::default();

        let probed =
            symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts)?;

        let mut format = probed.format;
        let track = format
            .default_track()
            .ok_or_else(|| anyhow::anyhow!("No default track"))?;
        let mut decoder =
            symphonia::default::get_codecs().make(&track.codec_params, &decoder_opts)?;

        let track_id = track.id;
        let src_sample_rate = track.codec_params.sample_rate.unwrap_or(48000);
        let channels = track
            .codec_params
            .channels
            .map(|c| c.count() as u16)
            .unwrap_or(2);

        let mut sample_buf = None;
        let mut all_samples = Vec::new();

        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(symphonia::core::errors::Error::ResetRequired) => {
                    decoder.reset();
                    continue;
                }
                Err(symphonia::core::errors::Error::IoError(err)) => {
                    if err.kind() == std::io::ErrorKind::UnexpectedEof {
                        break;
                    }
                    break;
                }
                Err(_) => break,
            };

            if packet.track_id() != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(audio_buf) => {
                    if sample_buf.is_none() {
                        let spec = *audio_buf.spec();
                        let duration = audio_buf.capacity() as u64;
                        sample_buf = Some(SampleBuffer::<f32>::new(duration, spec));
                    }
                    if let Some(buf) = &mut sample_buf {
                        buf.copy_interleaved_ref(audio_buf);
                        all_samples.extend_from_slice(buf.samples());
                    }
                }
                Err(symphonia::core::errors::Error::DecodeError(_)) => {}
                Err(_) => break,
            }
        }

        Ok((all_samples, channels, src_sample_rate))
    }

    /// 자체 WAV 리더로 끝까지 읽는다(27채널 이상·RF64, 안정성 ⑨).
    fn read_with_own_reader(
        path: &std::path::Path,
        info: crate::audio::wav_reader::WavInfo,
    ) -> anyhow::Result<(Vec<f32>, u16, u32)> {
        let channels = info.channels;
        let rate = info.sample_rate;
        let mut reader = crate::audio::wav_reader::WavReader::open_with(path, info)?;
        let mut all = Vec::with_capacity((reader.info().frames as usize).saturating_mul(channels as usize));
        let mut block = Vec::new();
        loop {
            let n = reader.read(4096, &mut block)?;
            if n == 0 {
                break;
            }
            all.extend_from_slice(&block);
        }
        Ok((all, channels, rate))
    }
}

/// 메모리에 미리 올리는 음원의 최대 표본 수(f32 4바이트 → 약 100MB). 넘는 파일은 재생할 때 디스크에서 읽는다
/// (사용자 결정 2026-11-24 D3, 예전에는 미리 읽기가 실패해 그 트랙이 나지 않았다).
pub const PRELOAD_MAX_SAMPLES: u64 = 25_000_000;

/// 메모리에 올렸을 때의 표본 수(장치 샘플레이트로 바꾼 뒤). 머리말만 읽는다. 길이·채널을 모르면 None.
pub fn estimated_samples(path: &std::path::Path, target_sample_rate: u32) -> Option<u64> {
    let (frames, rate, channels) = match crate::audio::wav_reader::choose(path) {
        crate::audio::wav_reader::Choice::Own(info) => (info.frames, info.sample_rate, info.channels as u64),
        crate::audio::wav_reader::Choice::Unsupported(_) => return None,
        crate::audio::wav_reader::Choice::Symphonia => {
            let file = File::open(path).ok()?;
            let mss = MediaSourceStream::new(Box::new(file), Default::default());
            let mut hint = Hint::new();
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                hint.with_extension(&ext.to_lowercase());
            }
            let probed = symphonia::default::get_probe()
                .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
                .ok()?;
            let params = &probed.format.default_track()?.codec_params;
            (params.n_frames?, params.sample_rate?, params.channels?.count() as u64)
        }
    };
    let out_frames = if target_sample_rate > 0 && rate > 0 && target_sample_rate != rate {
        (frames as f64 * target_sample_rate as f64 / rate as f64).round() as u64
    } else {
        frames
    };
    Some(out_frames * channels)
}

/// 메모리에 올리기에 너무 큰가([`PRELOAD_MAX_SAMPLES`]). 길이를 모르면 false(지금처럼 올려 본다).
pub fn too_big_for_memory(path: &std::path::Path, target_sample_rate: u32) -> bool {
    estimated_samples(path, target_sample_rate).is_some_and(|n| n > PRELOAD_MAX_SAMPLES)
}

/// 이 트랙을 디스크에서 읽어 재생하는가: 루프, 스트리밍 켬, 또는 메모리에 올리기에 너무 큰 파일.
pub fn plays_from_disk(is_loop: bool, is_streaming: bool, path: &std::path::Path, target_sample_rate: u32) -> bool {
    is_loop || is_streaming || too_big_for_memory(path, target_sample_rate)
}

pub struct SoundInstance {
    pub instance_id: u64,
    pub id: u32,
    pub room_id: u32,
    pub track_id_str: String,
    pub data: Option<Arc<SoundData>>,
    pub stream_receiver: Option<rtrb::Consumer<Vec<f32>>>,
    pub streamer: Option<crate::audio::streaming::DiskStreamer>,
    pub stream_buffer: Vec<f32>,
    pub stream_sample_rate: u32,
    pub stream_channels: u16,
    pub cursor: f64,
    pub volume: f32,
    pub is_loop: bool,
    pub is_playing: bool,
    pub is_stopping: bool,
    pub output_channel: usize,
    pub output_stereo: bool,
    /// 출력 채널을 바꾸기 전의 경로(SetTrackOutput). `output_fade`가 1이 될 때까지 이 경로는 줄이고 새 경로는
    /// 키운다(3법칙 3). 예전에는 소리가 옛 채널에서 끊기고 새 채널에서 바로 시작해 양쪽에서 딸깍 소리가 났다.
    pub prev_output_channel: usize,
    pub prev_output_stereo: bool,
    /// 새 출력 경로의 비중(0 -> 1). 1이면 옛 경로를 쓰지 않는다.
    pub output_fade: f32,
    pub fade_weight: f32, // 0.0 to 1.0
    pub volume_smoother: crate::audio::dsp::dsp_utils::GainSmoother,
    pub last_samples: Vec<f32>,
    pub anti_click_multiplier: f32,
    /// 스트리밍: 지금 `stream_buffer` 앞까지 지나간 프레임 수(시작 위치 포함). 미리 로드한 데이터는 0.
    pub position_base_frames: f64,
    /// 루프 한 바퀴 길이(프레임). 모르면 None이라 위치를 바퀴 안으로 접지 못한다.
    pub loop_len_frames: Option<f64>,
    /// 그룹 시작 묶음 번호(0이면 그룹 아님, docs/superpowers/specs/2026-11-12-group-start-design.md).
    pub group_id: u64,
    /// StartGroup을 기다리는 중. 이 동안은 소리·위치·페이드가 움직이지 않는다.
    pub armed: bool,
    /// 그룹 인스턴스가 스트림이 비어 따라가지 못한 프레임 수. 묶음이 오면 이만큼 건너뛰어 그룹 위치로 돌아간다.
    pub lag_frames: f64,
    /// 건너뛴 직후 다시 들어오는 페이드(0 -> 1, 딸깍 방지, 3법칙 3). 평소에는 1.
    pub rejoin_gain: f32,
    /// 트랙 출력이 켠 그룹과 맞는 정도(0 -> 1, core::output_groups). 맞지 않게 되면 0으로, 다시 맞으면 1로
    /// `OUTPUT_RAMP_S`에 걸쳐 옮긴다(3법칙 3). 음수면 아직 정하지 않음 — 믹서가 첫 블록에 바로 0이나 1로 정한다
    /// (시작하자마자 맞지 않는 트랙이 새어 나오지 않게).
    pub route_gain: f32,
    /// 현재 경로가 연결 표인가(false면 `output_channel`·`output_stereo`의 매핑 경로). 매트릭스 라우팅 설계 6.2.
    pub table_active: bool,
    /// 직전 경로가 표였나(`output_fade` < 1 동안만 뜻이 있다).
    pub prev_table_active: bool,
    /// 현재·직전·대기 표. 셋 다 `MAX_TRACK_ROUTES` 용량으로 `new()`에서 잡고, 콜백은 clear·extend_from_slice·
    /// mem::swap만 한다(할당 없음, 3법칙 1).
    pub routes: Vec<crate::audio::route::RouteEntry>,
    pub prev_routes: Vec<crate::audio::route::RouteEntry>,
    pub pending_routes: Vec<crate::audio::route::RouteEntry>,
    /// 페이드 중에 온 경로 변경. 페이드가 끝난 블록 시작에 적용한다(`promote_pending`).
    pub pending_path: Option<PendingPath>,
}

/// 페이드 중에 온 경로 변경(설계 6.3). 표면 칸은 `pending_routes`에 있다.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PendingPath {
    pub is_table: bool,
    pub output_channel: usize,
    pub output_stereo: bool,
}

impl SoundInstance {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instance_id: u64,
        id: u32,
        room_id: u32,
        track_id_str: String,
        data: Option<Arc<SoundData>>,
        mut streamer: Option<crate::audio::streaming::DiskStreamer>,
        stream_sample_rate: u32,
        stream_channels: u16,
        is_loop: bool,
        volume: f32,
        output_channel: usize,
        output_stereo: bool,
        // 아래 두 인자는 옛 3D Object 경로(트랙을 좌표로 패닝)가 쓰던 것이다. 그 경로는 지웠고 값은 쓰지 않는다.
        // 시험 20여 개 파일이 위치 인자로 부르고 있어 자리는 남긴다(매트릭스 라우팅 구현 때 함께 정리).
        _current_position: Option<crate::common::config::Point3D>,
        _spatial_channel_capacity: usize,
    ) -> Self {
        let loop_len_frames = if is_loop {
            streamer.as_ref().and_then(|s| s.loop_len_frames).or_else(|| {
                data.as_ref().map(|d| (d.samples.len() / d.channels.max(1) as usize) as f64)
            })
        } else {
            None
        };
        let mut smoother = crate::audio::dsp::dsp_utils::GainSmoother::new(1.0, 0.01);
        smoother.set_target(volume);

        let stream_receiver = streamer.as_mut().and_then(|s| s.chunk_receiver.take());

        Self {
            instance_id,
            id,
            room_id,
            track_id_str,
            data,
            stream_receiver,
            streamer,
            stream_buffer: Vec::with_capacity(131072),
            stream_sample_rate,
            stream_channels,
            cursor: 0.0,
            volume,
            is_loop,
            is_playing: true,
            is_stopping: false,
            output_channel,
            output_stereo,
            prev_output_channel: output_channel,
            prev_output_stereo: output_stereo,
            output_fade: 1.0,
            fade_weight: 0.0,
            volume_smoother: smoother,
            last_samples: vec![0.0; stream_channels as usize],
            anti_click_multiplier: 1.0,
            position_base_frames: 0.0,
            loop_len_frames,
            group_id: 0,
            armed: false,
            lag_frames: 0.0,
            rejoin_gain: 1.0,
            route_gain: -1.0,
            table_active: false,
            prev_table_active: false,
            routes: Vec::with_capacity(crate::audio::route::MAX_TRACK_ROUTES),
            prev_routes: Vec::with_capacity(crate::audio::route::MAX_TRACK_ROUTES),
            pending_routes: Vec::with_capacity(crate::audio::route::MAX_TRACK_ROUTES),
            pending_path: None,
        }
    }

    /// 재생 전에(오디오 스레드 밖) 이 인스턴스를 표 경로로 시작하게 한다. 페이드는 걸지 않는다. 용량을 넘는 칸은
    /// 앞에서부터 용량만큼만 받는다.
    pub fn set_initial_routes(&mut self, entries: &[crate::audio::route::RouteEntry]) {
        let n = entries.len().min(self.routes.capacity());
        self.routes.clear();
        self.routes.extend_from_slice(&entries[..n]);
        self.table_active = true;
    }

    /// 경로를 표로 바꾼다(콜백에서 부른다, 할당 없음). 페이드 중이면 대기에 두었다가 페이드가 끝나면 바꾼다
    /// (설계 6.3: 몰린 변경에도 들리던 경로는 언제나 10ms에 걸쳐 준다).
    pub fn request_table_path(&mut self, entries: &[crate::audio::route::RouteEntry]) {
        let n = entries.len().min(self.pending_routes.capacity());
        let entries = &entries[..n];
        let same = match self.pending_path {
            Some(p) => p.is_table && self.pending_routes.as_slice() == entries,
            None => self.table_active && self.routes.as_slice() == entries,
        };
        if same {
            return;
        }
        if self.output_fade >= 1.0 {
            self.begin_switch();
            self.routes.clear();
            self.routes.extend_from_slice(entries);
            self.table_active = true;
        } else {
            self.pending_routes.clear();
            self.pending_routes.extend_from_slice(entries);
            self.pending_path =
                Some(PendingPath { is_table: true, output_channel: self.output_channel, output_stereo: self.output_stereo });
        }
    }

    /// 경로를 매핑(`output_channel`·`output_stereo`)으로 바꾼다. 규칙은 [`Self::request_table_path`]와 같다.
    pub fn request_legacy_path(&mut self, output_channel: usize, output_stereo: bool) {
        let same = match self.pending_path {
            Some(p) => !p.is_table && p.output_channel == output_channel && p.output_stereo == output_stereo,
            None => !self.table_active && self.output_channel == output_channel && self.output_stereo == output_stereo,
        };
        if same {
            return;
        }
        if self.output_fade >= 1.0 {
            self.begin_switch();
            self.output_channel = output_channel;
            self.output_stereo = output_stereo;
            self.table_active = false;
        } else {
            self.pending_path = Some(PendingPath { is_table: false, output_channel, output_stereo });
        }
    }

    /// 페이드가 끝났고 대기한 변경이 있으면 적용한다. 믹서가 블록 시작에 재생 중인 인스턴스마다 부른다.
    pub fn promote_pending(&mut self) {
        if self.output_fade < 1.0 {
            return;
        }
        let Some(p) = self.pending_path.take() else {
            return;
        };
        let same = if p.is_table {
            self.table_active && self.routes == self.pending_routes
        } else {
            !self.table_active && self.output_channel == p.output_channel && self.output_stereo == p.output_stereo
        };
        if same {
            return;
        }
        self.begin_switch();
        if p.is_table {
            // begin_switch가 현재 표를 직전으로 옮겼다. 대기 표를 현재로 가져온다(남는 배열은 다음 대기에 다시 쓴다).
            std::mem::swap(&mut self.routes, &mut self.pending_routes);
            self.table_active = true;
        } else {
            self.output_channel = p.output_channel;
            self.output_stereo = p.output_stereo;
            self.table_active = false;
        }
    }

    /// 현재 경로를 직전으로 옮기고 페이드를 시작한다(새 경로는 부른 쪽이 넣는다).
    fn begin_switch(&mut self) {
        std::mem::swap(&mut self.routes, &mut self.prev_routes);
        self.prev_table_active = self.table_active;
        self.prev_output_channel = self.output_channel;
        self.prev_output_stereo = self.output_stereo;
        self.output_fade = 0.0;
    }

    /// 그룹 `group_id`(0이 아님)의 대기 인스턴스로 표시한다. StartGroup이 오면 그룹과 함께 시작한다.
    pub fn arm_for_group(&mut self, group_id: u64) {
        self.group_id = group_id;
        self.armed = true;
    }

    /// 파일 기준 재생 위치(초). 루프면 한 바퀴 안의 위치다. 그룹 인스턴스가 스트림이 비어 밀렸으면
    /// 있어야 할 자리(밀린 프레임 포함)를 돌려준다 — 재시작 스냅샷이 그룹 위치로 이어 가게.
    pub fn position_seconds(&self) -> f64 {
        let mut frames = self.position_base_frames + self.cursor + self.lag_frames;
        if let Some(len) = self.loop_len_frames {
            if len > 0.0 {
                frames %= len;
            }
        }
        frames / self.stream_sample_rate.max(1) as f64
    }

    /// 파일의 `seconds` 지점부터 시작하게 한다(재시작 복원). 스트리밍이면
    /// `DiskStreamer::new_at`이 그만큼 건너뛰고 시작해야 위치가 맞는다.
    pub fn set_start_position(&mut self, seconds: f64) {
        let frames = seconds.max(0.0) * self.stream_sample_rate as f64;
        if self.stream_receiver.is_some() {
            self.position_base_frames = frames;
        } else {
            self.cursor = frames;
        }
    }
}
