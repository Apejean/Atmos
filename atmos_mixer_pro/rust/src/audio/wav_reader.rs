//! 자체 WAV 리더(안정성 ⑨, 설계 `docs/superpowers/specs/2026-11-24-wav-reader-design.md`).
//!
//! symphonia 0.5.5는 27채널 이상 WAV(채널 위치 플래그가 26개)와 RF64(4GB를 넘는 WAV)를 열지 못한다. 그런 파일만
//! 이 리더로 읽고, 그 밖의 파일은 지금처럼 symphonia로 읽는다(회귀 방지, 확정된 결정). 형식은 PCM 정수
//! 16·24·32bit와 float 32bit(사용자 결정 2026-11-24 D2). 채널 마스크는 보지 않는다 — 파일 채널 n은 그대로 n번째다.
//!
//! 디코더 스레드·미리 읽기에서만 부른다(파일 I/O). 오디오 콜백에서는 부르지 않는다.

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// 한 파일의 최대 채널 수(사용자 결정 2026-11-24 D1). 매트릭스 라우팅의 파일 채널 상한도 이 값을 쓴다.
pub const MAX_FILE_CHANNELS: u16 = 256;
/// symphonia가 여는 최대 채널 수. 이보다 많거나 RF64면 이 리더를 쓴다.
pub const SYMPHONIA_MAX_CHANNELS: u16 = 26;
/// 읽기 버퍼 크기. 64채널 24bit 1024프레임 묶음(약 192KB)을 몇 번 담는다.
const READ_BUFFER_BYTES: usize = 1 << 20;

/// 표본 형식.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcmFormat {
    I16,
    I24,
    I32,
    F32,
}

impl PcmFormat {
    pub fn bytes(self) -> usize {
        match self {
            PcmFormat::I16 => 2,
            PcmFormat::I24 => 3,
            PcmFormat::I32 | PcmFormat::F32 => 4,
        }
    }
}

/// 머리말에서 읽은 것.
#[derive(Debug, Clone, PartialEq)]
pub struct WavInfo {
    pub channels: u16,
    pub sample_rate: u32,
    pub format: PcmFormat,
    /// 프레임 수. 머리말보다 파일이 짧으면 실제 길이까지로 줄인 값.
    pub frames: u64,
    /// 파일 안 data 시작 위치(바이트).
    pub data_offset: u64,
    pub rf64: bool,
}

impl WavInfo {
    pub fn block_align(&self) -> u64 {
        self.channels as u64 * self.format.bytes() as u64
    }
}

/// 이 파일을 무엇으로 읽나.
#[derive(Debug, Clone, PartialEq)]
pub enum Choice {
    /// symphonia(지금 길). WAV가 아니거나 26채널 이하 RIFF.
    Symphonia,
    /// 이 리더.
    Own(WavInfo),
    /// 이 리더가 맡을 파일(27채널 이상·RF64)인데 읽을 수 없다. 사람이 읽는 이유.
    Unsupported(String),
}

/// 머리말만 읽어 정한다. 파일을 못 열면 Symphonia(그쪽이 지금처럼 오류를 낸다).
pub fn choose(path: &Path) -> Choice {
    let Ok(file) = File::open(path) else {
        return Choice::Symphonia;
    };
    let mut r = BufReader::new(file);
    match parse(&mut r) {
        Parsed::NotWave => Choice::Symphonia,
        Parsed::Header(h) => {
            if !h.rf64 && h.channels <= SYMPHONIA_MAX_CHANNELS {
                return Choice::Symphonia;
            }
            match h.validate(file_len(path)) {
                Ok(info) => Choice::Own(info),
                Err(msg) => Choice::Unsupported(msg),
            }
        }
        Parsed::Broken(msg) => {
            // 깨진 머리말이라도 27채널 이상·RF64로 보이면 이유를 알린다. 그 밖은 symphonia에 맡긴다.
            if msg.big { Choice::Unsupported(msg.text) } else { Choice::Symphonia }
        }
    }
}

/// [`choose`]가 Own을 준 파일을 읽는다.
pub struct WavReader {
    file: BufReader<File>,
    info: WavInfo,
    /// 다음에 읽을 프레임.
    pos: u64,
    raw: Vec<u8>,
}

impl WavReader {
    /// 이 리더가 맡는 파일만 연다(26채널 이하 RIFF·WAV 아님·읽을 수 없는 형식은 오류).
    pub fn open(path: &Path) -> anyhow::Result<WavReader> {
        match choose(path) {
            Choice::Own(info) => Self::open_with(path, info),
            Choice::Unsupported(msg) => Err(anyhow::anyhow!(msg)),
            Choice::Symphonia => Err(anyhow::anyhow!("자체 WAV 리더 대상이 아니다(27채널 미만 RIFF이거나 WAV가 아님)")),
        }
    }

    /// 이미 읽은 머리말로 연다(다시 열기).
    pub fn open_with(path: &Path, info: WavInfo) -> anyhow::Result<WavReader> {
        let file = File::open(path)?;
        let mut file = BufReader::with_capacity(READ_BUFFER_BYTES, file);
        file.seek(SeekFrom::Start(info.data_offset))?;
        Ok(WavReader { file, info, pos: 0, raw: Vec::new() })
    }

    pub fn info(&self) -> &WavInfo {
        &self.info
    }

    /// 다음에 읽을 프레임.
    pub fn position(&self) -> u64 {
        self.pos
    }

    /// 최대 `max_frames`를 f32 인터리브로 `out`에 채운다(`out`을 비우고 다시 쓴다). 읽은 프레임 수, 0이면 파일 끝.
    /// 머리말보다 파일이 짧아 중간에 끝나면 읽은 만큼만 돌려주고 다음은 0이다.
    pub fn read(&mut self, max_frames: usize, out: &mut Vec<f32>) -> std::io::Result<usize> {
        out.clear();
        let left = self.info.frames.saturating_sub(self.pos);
        let want = (max_frames as u64).min(left) as usize;
        if want == 0 {
            return Ok(0);
        }
        let block = self.info.block_align() as usize;
        self.raw.resize(want * block, 0);
        let mut got = 0;
        while got < self.raw.len() {
            match self.file.read(&mut self.raw[got..]) {
                Ok(0) => break,
                Ok(n) => got += n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        let frames = got / block;
        if frames < want {
            // 파일이 머리말보다 짧다(파일을 연 뒤 잘렸다). 남은 것은 없는 것으로 본다.
            self.info.frames = self.pos + frames as u64;
        }
        let samples = frames * self.info.channels as usize;
        out.reserve(samples);
        convert(self.info.format, &self.raw[..frames * block], out);
        self.pos += frames as u64;
        Ok(frames)
    }

    /// `frame`으로 간다(PCM이라 샘플 단위로 정확). 끝보다 뒤면 끝.
    pub fn seek(&mut self, frame: u64) -> std::io::Result<()> {
        let frame = frame.min(self.info.frames);
        self.file
            .seek(SeekFrom::Start(self.info.data_offset + frame * self.info.block_align()))?;
        self.pos = frame;
        Ok(())
    }
}

/// symphonia와 같은 값으로 바꾼다(정수는 2^(비트−1)로 나눔).
fn convert(format: PcmFormat, raw: &[u8], out: &mut Vec<f32>) {
    match format {
        PcmFormat::I16 => out.extend(
            raw.as_chunks::<2>().0.iter().map(|b| i16::from_le_bytes(*b) as f32 / 32_768.0),
        ),
        PcmFormat::I24 => out.extend(raw.as_chunks::<3>().0.iter().map(|b| {
            let v = (i32::from_le_bytes([0, b[0], b[1], b[2]])) >> 8;
            v as f32 / 8_388_608.0
        })),
        PcmFormat::I32 => out.extend(
            raw.as_chunks::<4>()
                .0
                .iter()
                .map(|b| (i32::from_le_bytes(*b) as f64 / 2_147_483_648.0) as f32),
        ),
        PcmFormat::F32 => out.extend(
            raw.as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes(*b)),
        ),
    }
}

fn file_len(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(u64::MAX)
}

/// 머리말 덩어리에서 모은 값(검사 전).
struct Header {
    rf64: bool,
    format_tag: u16,
    sub_format: Option<u16>,
    channels: u16,
    sample_rate: u32,
    block_align: u16,
    bits: u16,
    data_offset: u64,
    data_len: u64,
}

struct BrokenMsg {
    /// 27채널 이상이나 RF64로 보이는 파일인가.
    big: bool,
    text: String,
}

enum Parsed {
    NotWave,
    Header(Header),
    Broken(BrokenMsg),
}

impl Header {
    fn validate(&self, file_len: u64) -> Result<WavInfo, String> {
        if self.channels == 0 {
            return Err("WAV 채널 수가 0이다".to_string());
        }
        if self.channels > MAX_FILE_CHANNELS {
            return Err(format!(
                "채널이 너무 많은 WAV다: {}채널(최대 {}채널)",
                self.channels, MAX_FILE_CHANNELS
            ));
        }
        let tag = match self.format_tag {
            0xFFFE => self.sub_format.unwrap_or(0),
            t => t,
        };
        let format = match (tag, self.bits) {
            (1, 16) => PcmFormat::I16,
            (1, 24) => PcmFormat::I24,
            (1, 32) => PcmFormat::I32,
            (3, 32) => PcmFormat::F32,
            (3, 64) => return Err("지원하지 않는 WAV 형식: float 64bit(PCM 16·24·32bit, float 32bit만 읽는다)".to_string()),
            (1, b) => {
                return Err(format!("지원하지 않는 WAV 형식: PCM {b}bit(PCM 16·24·32bit, float 32bit만 읽는다)"))
            }
            (t, b) => {
                return Err(format!(
                    "지원하지 않는 WAV 형식: 형식 번호 {t}, {b}bit(PCM 16·24·32bit, float 32bit만 읽는다)"
                ))
            }
        };
        let block = self.channels as u64 * format.bytes() as u64;
        if self.block_align as u64 != block {
            return Err(format!(
                "WAV 머리말이 맞지 않다: 한 프레임 {}바이트인데 채널 {} × {}바이트",
                self.block_align,
                self.channels,
                format.bytes()
            ));
        }
        let available = file_len.saturating_sub(self.data_offset);
        let frames = self.data_len.min(available) / block;
        Ok(WavInfo {
            channels: self.channels,
            sample_rate: self.sample_rate,
            format,
            frames,
            data_offset: self.data_offset,
            rf64: self.rf64,
        })
    }
}

fn read_u16(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}
fn read_u32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}
fn read_u64(b: &[u8], at: usize) -> u64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&b[at..at + 8]);
    u64::from_le_bytes(a)
}

/// RIFF/RF64(BW64) 머리말을 읽는다. 덩어리를 차례로 건너뛰며 ds64·fmt·data를 찾는다.
fn parse<R: Read + Seek>(r: &mut R) -> Parsed {
    let mut head = [0u8; 12];
    if r.read_exact(&mut head).is_err() {
        return Parsed::NotWave;
    }
    let rf64 = match &head[0..4] {
        b"RIFF" => false,
        b"RF64" | b"BW64" => true,
        _ => return Parsed::NotWave,
    };
    if &head[8..12] != b"WAVE" {
        return Parsed::NotWave;
    }
    let broken = |big: bool, text: &str| Parsed::Broken(BrokenMsg { big, text: text.to_string() });

    let mut ds64_data: Option<u64> = None;
    let mut fmt: Option<(u16, Option<u16>, u16, u32, u16, u16)> = None;
    let mut data: Option<(u64, u32)> = None;
    let mut pos: u64 = 12;
    loop {
        let mut ch = [0u8; 8];
        if r.read_exact(&mut ch).is_err() {
            break;
        }
        let id = [ch[0], ch[1], ch[2], ch[3]];
        let size = read_u32(&ch, 4);
        let body_at = pos + 8;
        match &id {
            b"ds64" => {
                let mut b = vec![0u8; (size as usize).min(64)];
                if b.len() < 24 || r.read_exact(&mut b).is_err() {
                    return broken(true, "RF64 머리말이 깨졌다(ds64)");
                }
                ds64_data = Some(read_u64(&b, 8));
            }
            b"fmt " => {
                let mut b = vec![0u8; (size as usize).min(64)];
                if b.len() < 16 || r.read_exact(&mut b).is_err() {
                    return broken(rf64, "WAV 머리말이 깨졌다(fmt)");
                }
                let tag = read_u16(&b, 0);
                let sub = if tag == 0xFFFE && b.len() >= 26 { Some(read_u16(&b, 24)) } else { None };
                fmt = Some((tag, sub, read_u16(&b, 2), read_u32(&b, 4), read_u16(&b, 12), read_u16(&b, 14)));
            }
            b"data" => {
                data = Some((body_at, size));
            }
            _ => {}
        }
        if fmt.is_some() && data.is_some() {
            break;
        }
        // 다음 덩어리: 크기 + 홀수면 채움 1바이트. data가 fmt보다 앞이면 data를 건너뛴다(RF64 data는 ds64 크기).
        let skip = if &id == b"data" && size == 0xFFFF_FFFF {
            match ds64_data {
                Some(n) => n,
                None => return broken(true, "RF64 data 크기를 모른다(ds64 없음)"),
            }
        } else {
            size as u64
        };
        pos = body_at + skip + (skip & 1);
        if r.seek(SeekFrom::Start(pos)).is_err() {
            break;
        }
    }
    let Some((tag, sub, channels, sample_rate, block_align, bits)) = fmt else {
        return broken(rf64, "WAV 머리말에 fmt가 없다");
    };
    let big = rf64 || channels > SYMPHONIA_MAX_CHANNELS;
    let Some((data_offset, size)) = data else {
        return broken(big, "WAV에 data가 없다");
    };
    let data_len = if size == 0xFFFF_FFFF && rf64 {
        match ds64_data {
            Some(n) => n,
            None => return broken(true, "RF64 data 크기를 모른다(ds64 없음)"),
        }
    } else {
        size as u64
    };
    Parsed::Header(Header {
        rf64,
        format_tag: tag,
        sub_format: sub,
        channels,
        sample_rate,
        block_align,
        bits,
        data_offset,
        data_len,
    })
}
