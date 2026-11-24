//! 트랙 연결 표(매트릭스 라우팅, 설계 `docs/superpowers/specs/2026-11-14-matrix-routing-design.md` 6.1·개정 3).
//!
//! 설정의 표(`TrackConfig.routes`)를 엔진이 쓰는 칸으로 검증·변환한다. 할당하므로 **오디오 스레드 밖에서만** 부른다.
//! 콜백은 인스턴스가 미리 잡은 배열에 복사한 결과만 쓴다(3법칙 1·2).

use crate::common::config::TrackRoute;

/// 한 트랙의 연결 표 칸 수 상한. 인스턴스는 이 용량으로 미리 잡는다(콜백 안 할당 금지, 설계 D11).
pub const MAX_TRACK_ROUTES: usize = 512;

/// 엔진이 쓰는 칸(검증·변환 끝).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RouteEntry {
    /// 파일 채널(0부터).
    pub src: u16,
    /// 출력 채널(0부터).
    pub out: u16,
    /// 선형 음량(0 초과 1 이하).
    pub gain: f32,
}

/// 설정의 표를 엔진 칸으로 바꾼 결과와 버린 칸 수.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ResolvedRoutes {
    /// 길이 <= [`MAX_TRACK_ROUTES`], (out, src) 오름차순.
    pub entries: Vec<RouteEntry>,
    pub dropped_bad_gain: usize,
    pub dropped_file_channel: usize,
    pub dropped_output: usize,
    pub dropped_duplicate: usize,
    pub dropped_over_limit: usize,
}

impl ResolvedRoutes {
    /// 버린 칸의 합.
    pub fn dropped(&self) -> usize {
        self.dropped_bad_gain
            + self.dropped_file_channel
            + self.dropped_output
            + self.dropped_duplicate
            + self.dropped_over_limit
    }
}

/// 표를 엔진 칸으로 바꾼다. `file_channels`를 알면 그 밖의 파일 채널 칸을 버린다(모르면 파일 채널 상한
/// `wav_reader::MAX_FILE_CHANNELS`까지 둔다 — 콜백이 실제 채널 수로 다시 거른다).
///
/// 규칙(순서대로, 설계 6.1):
/// 1. 음량이 유한하지 않거나 0 이하면 버린다. 1을 넘으면 1로 자른다(증폭 없음, D3).
/// 2. 파일 채널이 파일 채널 수(또는 상한) 이상이면 버린다.
/// 3. 출력 채널이 게이트 표 길이(4096) 이상이면 버린다. 장치·믹서 폭 밖인지는 보지 않는다(엔진이 다른 폭으로
///    다시 떠도 표가 맞게 — 콜백의 경계 검사가 맡는다).
/// 4. 같은 (파일 채널, 출력 채널)이 또 나오면 뒤의 것을 버린다.
/// 5. (출력, 파일) 오름차순으로 정렬하고 [`MAX_TRACK_ROUTES`]를 넘는 뒤쪽을 버린다.
pub fn resolve_routes(routes: &[TrackRoute], file_channels: Option<usize>) -> ResolvedRoutes {
    let file_limit = file_channels
        .unwrap_or(crate::audio::wav_reader::MAX_FILE_CHANNELS as usize)
        .min(crate::audio::wav_reader::MAX_FILE_CHANNELS as usize);
    let output_limit = crate::core::state::GLOBAL_STATE.enabled_channels.len().min(u16::MAX as usize);
    let mut out = ResolvedRoutes::default();
    let mut seen = std::collections::HashSet::new();
    let mut entries = Vec::with_capacity(routes.len().min(MAX_TRACK_ROUTES * 2));
    for r in routes {
        if !r.gain.is_finite() || r.gain <= 0.0 {
            out.dropped_bad_gain += 1;
            continue;
        }
        if r.file_channel as usize >= file_limit {
            out.dropped_file_channel += 1;
            continue;
        }
        if r.output_channel as usize >= output_limit {
            out.dropped_output += 1;
            continue;
        }
        if !seen.insert((r.file_channel, r.output_channel)) {
            out.dropped_duplicate += 1;
            continue;
        }
        entries.push(RouteEntry { src: r.file_channel as u16, out: r.output_channel as u16, gain: r.gain.min(1.0) });
    }
    entries.sort_by_key(|e| (e.out, e.src));
    if entries.len() > MAX_TRACK_ROUTES {
        out.dropped_over_limit = entries.len() - MAX_TRACK_ROUTES;
        entries.truncate(MAX_TRACK_ROUTES);
    }
    out.entries = entries;
    out
}
