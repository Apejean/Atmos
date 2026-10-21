//! 헤드폰 바이노럴 미리듣기가 쓰는 HRTF(HRIR) 측정 데이터셋.
//!
//! 앱에 내장하는 MIT KEMAR 데이터(`assets/hrtf/mit_kemar_normal_pinna.hrtfset`)를 읽는다.
//! 원본은 SOFA(AES69, SimpleFreeFieldHRIR) 파일이고, 개발용 변환 도구(`tool/hrtf_convert`, 제품에
//! 포함하지 않음)가 같은 숫자를 그대로 이 형식으로 옮겨 둔다. 제품은 SOFA 파서 없이 이 형식만
//! 읽는다. 출처·사용 조건·변환 기록은 `assets/hrtf/README.md`.
//!
//! 파일 형식(모두 리틀 엔디언, f32는 IEEE 754 비트 그대로):
//!
//! | 오프셋 | 크기 | 내용 |
//! |---|---|---|
//! | 0 | 8 | 매직 `ATMSHRTF` |
//! | 8 | 4 | 형식 판 u32 = 1 |
//! | 12 | 4 | 샘플레이트 f32(Hz) |
//! | 16 | 4 | 측정점 수 M u32 |
//! | 20 | 4 | 수신기(귀) 수 R u32 = 2 |
//! | 24 | 4 | IR 길이 N u32(샘플) |
//! | 28 | M×3×4 | 측정 위치 (방위각 도, 고도각 도, 거리 m) — SOFA SourcePosition(구면) |
//! | 28+M×12 | M×R×N×4 | IR — SOFA Data.IR와 같은 M×R×N 순서(측정점마다 왼쪽 귀 N개, 오른쪽 귀 N개) |

use anyhow::{bail, ensure, Context, Result};

/// 파일 맨 앞 8바이트.
pub const HRTF_SET_MAGIC: [u8; 8] = *b"ATMSHRTF";
/// 이 코드가 읽는 형식 판.
pub const HRTF_SET_VERSION: u32 = 1;
const HEADER_LEN: usize = 28;

/// 측정 위치(구면 좌표).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HrtfPosition {
    /// 방위각(도).
    pub azimuth: f32,
    /// 고도각(도).
    pub elevation: f32,
    /// 거리(m).
    pub distance: f32,
}

impl HrtfPosition {
    pub fn new(azimuth: f32, elevation: f32, distance: f32) -> Self {
        Self { azimuth, elevation, distance }
    }

    /// 두 방향 사이의 대원 각거리(도). 거리 성분은 보지 않는다.
    ///
    /// 하버사인 공식: 방위각을 경도, 고도각을 위도로 보고
    /// a = sin²(Δ고도/2) + cos(고도₁)·cos(고도₂)·sin²(Δ방위각/2), 각 = 2·atan2(√a, √(1−a)).
    /// 반올림으로 a가 [0, 1]을 살짝 벗어나면 √(1−a)가 NaN이 되므로 그 범위로 자른다.
    pub fn angular_distance(&self, other: &HrtfPosition) -> f32 {
        let (az1, el1) = (self.azimuth.to_radians(), self.elevation.to_radians());
        let (az2, el2) = (other.azimuth.to_radians(), other.elevation.to_radians());
        let half_del = ((el2 - el1) / 2.0).sin();
        let half_daz = ((az2 - az1) / 2.0).sin();
        let a = (half_del * half_del + el1.cos() * el2.cos() * (half_daz * half_daz)).clamp(0.0, 1.0);
        (2.0 * a.sqrt().atan2((1.0 - a).sqrt())).to_degrees()
    }

    /// 측정점(self)에서 `target`까지의 조회 거리(m): 평균 반지름으로 잰 호의 길이와 반지름 차를
    /// 직각으로 합친다. 각거리나 반지름 차가 유한하지 않으면 무한대.
    ///
    /// 최근접 탐색(바이노럴의 3점 보간, [HrtfSet::find_nearest])이 모두 이 거리를 쓴다.
    pub fn lookup_distance(&self, target: &HrtfPosition) -> f32 {
        let angular_deg = self.angular_distance(target);
        if !angular_deg.is_finite() {
            return f32::INFINITY;
        }
        let radial = (self.distance - target.distance).abs();
        if !radial.is_finite() {
            return f32::INFINITY;
        }
        let mean_r = ((self.distance.abs() + target.distance.abs()) * 0.5).max(1e-3);
        let tangential = mean_r * angular_deg.to_radians();
        (tangential * tangential + radial * radial).sqrt()
    }
}

/// 메모리에 올린 HRTF 데이터셋. 보정(audio::hrtf_eq)이 IR·길이·샘플레이트를 바꾼다.
#[derive(Debug, Clone)]
pub struct HrtfSet {
    /// IR 샘플레이트(Hz).
    pub sample_rate: f32,
    /// 측정점 수(`positions.len()`과 같다).
    pub num_measurements: usize,
    /// 귀 하나의 IR 길이(샘플).
    pub ir_length: usize,
    pub positions: Vec<HrtfPosition>,
    /// [측정점 M × 귀 2(왼쪽, 오른쪽) × 샘플 N].
    pub impulse_responses: Vec<f32>,
}

/// 바이트를 앞에서부터 읽는다.
struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(len).context("HRTF 데이터 길이 계산이 넘침")?;
        let s = self.bytes.get(self.pos..end).context("HRTF 데이터가 중간에 끝남")?;
        self.pos = end;
        Ok(s)
    }

    fn u32(&mut self) -> Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }
}

impl HrtfSet {
    /// 내장 파일 바이트를 읽는다(엔진 초기화 때, 오디오 스레드 밖).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut c = Cursor { bytes, pos: 0 };
        if c.take(8)? != HRTF_SET_MAGIC {
            bail!("HRTF 데이터 형식이 아님(매직 불일치)");
        }
        let version = c.u32()?;
        ensure!(version == HRTF_SET_VERSION, "지원하지 않는 HRTF 형식 판 {version}");
        let sample_rate = c.f32()?;
        let m = c.u32()? as usize;
        let receivers = c.u32()? as usize;
        let n = c.u32()? as usize;
        ensure!(sample_rate.is_finite() && sample_rate > 0.0, "샘플레이트가 잘못됨: {sample_rate}");
        ensure!(receivers == 2, "귀(수신기) 수가 2가 아님: {receivers}");
        ensure!(m > 0 && n > 0, "측정점 수 {m} 또는 IR 길이 {n}이 0");

        let pos_count = m.checked_mul(3).context("측정점 수가 너무 큼")?;
        let ir_count = m
            .checked_mul(receivers)
            .and_then(|v| v.checked_mul(n))
            .context("IR 크기가 너무 큼")?;
        let expected = pos_count
            .checked_add(ir_count)
            .and_then(|v| v.checked_mul(4))
            .and_then(|v| v.checked_add(HEADER_LEN))
            .context("HRTF 데이터 크기가 너무 큼")?;
        ensure!(bytes.len() == expected, "HRTF 데이터 크기 {}바이트, 기대 {expected}바이트", bytes.len());

        let mut positions = Vec::with_capacity(m);
        for _ in 0..m {
            let (azimuth, elevation, distance) = (c.f32()?, c.f32()?, c.f32()?);
            positions.push(HrtfPosition { azimuth, elevation, distance });
        }
        let mut impulse_responses = Vec::with_capacity(ir_count);
        for _ in 0..ir_count {
            impulse_responses.push(c.f32()?);
        }
        Ok(Self { sample_rate, num_measurements: m, ir_length: n, positions, impulse_responses })
    }

    /// 측정점 `index`의 (위치, 왼쪽 귀 IR, 오른쪽 귀 IR). 내부 버퍼를 빌려 주므로 할당이 없다
    /// (오디오 스레드에서 쓴다). 범위 밖이면 None.
    pub fn get_hrtf_slices(&self, index: usize) -> Option<(HrtfPosition, &[f32], &[f32])> {
        let position = *self.positions.get(index)?;
        let n = self.ir_length;
        let start = index.checked_mul(2 * n)?;
        let left = self.impulse_responses.get(start..start + n)?;
        let right = self.impulse_responses.get(start + n..start + 2 * n)?;
        Some((position, left, right))
    }

    /// `target`에 가장 가까운 측정점의 (인덱스, 조회 거리). 거리가 같으면 앞선 측정점.
    /// 측정점이 없거나 모든 거리가 유한하지 않으면 (0, 무한대).
    pub fn find_nearest(&self, target: &HrtfPosition) -> (usize, f32) {
        let mut best = (0usize, f32::INFINITY);
        for (i, p) in self.positions.iter().enumerate() {
            let d = p.lookup_distance(target);
            if d < best.1 {
                best = (i, d);
            }
        }
        best
    }
}
