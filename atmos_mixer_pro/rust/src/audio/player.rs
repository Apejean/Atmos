use std::sync::Arc;

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

pub fn downsample_hi_res_if_needed(in_sample_rate: u32, samples: Vec<f32>, channels: usize) -> (Vec<f32>, u32) {
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
        if let Ok(file) = File::open(path) {
            let mss = MediaSourceStream::new(Box::new(file), Default::default());
            let mut hint = Hint::new();
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                hint.with_extension(&ext.to_lowercase());
            }
            let format_opts = FormatOptions::default();
            let metadata_opts = MetadataOptions::default();
            if let Ok(probed) = symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts) {
                if let Some(track) = probed.format.default_track() {
                    return track.codec_params.channels.map(|c| c.count() as u32).unwrap_or(2);
                }
            }
        }
        2
    }

    pub fn load_from_file(path: &std::path::Path) -> anyhow::Result<Self> {
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
        let sample_rate = track.codec_params.sample_rate.unwrap_or(48000);
        let channels = track
            .codec_params
            .channels
            .map(|c| c.count() as u16)
            .unwrap_or(2);
        let max_frames = track.codec_params.max_frames_per_packet.unwrap_or(4096);

        let mut sample_buf = None;
        let mut all_samples = Vec::new();
        let mut actual_channels = channels;

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
                    break; // Just break on any IO error (like EOF)
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
                        actual_channels = spec.channels.count() as u16;
                        let duration = std::cmp::max(audio_buf.capacity() as u64, max_frames);
                        sample_buf = Some(SampleBuffer::<f32>::new(duration, spec));
                    }

                    if let Some(buf) = &mut sample_buf {
                        buf.copy_interleaved_ref(audio_buf);
                        all_samples.extend_from_slice(buf.samples());
                    }
                }
                Err(symphonia::core::errors::Error::DecodeError(_)) => (),
                Err(_) => break,
            }
        }

        let (final_samples, final_sample_rate) = downsample_hi_res_if_needed(sample_rate, all_samples, actual_channels as usize);

        Ok(Self {
            samples: final_samples,
            channels: actual_channels,
            sample_rate: final_sample_rate,
        })
    }
}

pub struct SoundInstance {
    pub instance_id: u64,
    pub id: u32,
    pub room_id: u32,
    pub track_id_str: String,
    pub data: Option<Arc<SoundData>>,
    pub stream_receiver: Option<rtrb::Consumer<Vec<f32>>>,
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
    pub fade_weight: f32, // 0.0 to 1.0
    pub volume_smoother: crate::audio::dsp::dsp_utils::GainSmoother,
    pub last_samples: Vec<f32>,
    pub anti_click_multiplier: f32,
    pub current_position: Option<crate::common::config::Point3D>,
    pub spatial_gains: Vec<f32>,
    pub spatial_gains_target: Vec<f32>,
}

impl SoundInstance {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instance_id: u64,
        id: u32,
        room_id: u32,
        track_id_str: String,
        data: Option<Arc<SoundData>>,
        stream_receiver: Option<rtrb::Consumer<Vec<f32>>>,
        stream_sample_rate: u32,
        stream_channels: u16,
        is_loop: bool,
        volume: f32,
        output_channel: usize,
        output_stereo: bool,
        current_position: Option<crate::common::config::Point3D>,
    ) -> Self {
        let channels_usize = stream_channels as usize;
        Self {
            instance_id,
            id,
            room_id,
            track_id_str,
            data,
            stream_receiver,
            stream_buffer: Vec::new(),
            stream_sample_rate,
            stream_channels,
            cursor: 0.0,
            volume,
            is_loop,
            is_playing: true,
            is_stopping: false,
            output_channel,
            output_stereo,
            fade_weight: 0.0, // starts from 0 for fade in
            volume_smoother: crate::audio::dsp::dsp_utils::GainSmoother::new(volume, 0.005),
            last_samples: vec![0.0; channels_usize.max(1)],
            anti_click_multiplier: 1.0,
            current_position,
            spatial_gains: Vec::new(),
            spatial_gains_target: Vec::new(),
        }
    }
}
