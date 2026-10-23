//! 화면의 리샘플링 배지(2026-10-24 기능 점검). 예전 배지는 "44.1kHz → 48kHz"를 고정으로 보여 줬다.
//! 재생 중인 트랙의 원래 샘플레이트를 장치 샘플레이트와 비교해, 엔진이 HQ sinc로 바꾸고 있는 것만 센다.
use std::collections::HashMap;

/// 변환 중인 음원의 원래 샘플레이트들(작은 것부터, 중복 없음)과 그런 트랙 수.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResampleSummary {
    pub source_rates: Vec<u32>,
    pub track_count: u32,
}

/// [playing_track_ids]마다 [track_source_rates](트랙 → 원래 샘플레이트)를 찾아 [device_rate]와 다르면 센다.
/// 원래 샘플레이트를 모르는 트랙은 세지 않는다.
pub fn summarize(
    device_rate: u32,
    playing_track_ids: &[String],
    track_source_rates: &HashMap<String, u32>,
) -> ResampleSummary {
    let mut summary = ResampleSummary::default();
    for track_id in playing_track_ids {
        let Some(&rate) = track_source_rates.get(track_id) else {
            continue;
        };
        if rate != device_rate {
            summary.track_count += 1;
            if !summary.source_rates.contains(&rate) {
                summary.source_rates.push(rate);
            }
        }
    }
    summary.source_rates.sort_unstable();
    summary
}
