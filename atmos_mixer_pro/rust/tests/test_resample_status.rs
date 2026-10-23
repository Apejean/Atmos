//! 화면의 리샘플링 배지 요약(core::resample_status). 재생 중인 트랙 중 장치 샘플레이트와 다른 것만 센다.
use rust_lib_atmos_mixer_pro::core::resample_status::{summarize, ResampleSummary};
use std::collections::HashMap;

fn rates(pairs: &[(&str, u32)]) -> HashMap<String, u32> {
    pairs.iter().map(|(id, r)| (id.to_string(), *r)).collect()
}

fn ids(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn 장치와_다른_트랙만_원래_샘플레이트와_함께_센다() {
    let table = rates(&[("a", 44_100), ("b", 48_000), ("c", 96_000), ("e", 44_100)]);
    // d는 원래 샘플레이트를 모른다(세지 않는다).
    let summary = summarize(48_000, &ids(&["a", "b", "c", "d", "e"]), &table);
    assert_eq!(summary, ResampleSummary { source_rates: vec![44_100, 96_000], track_count: 3 });
}

#[test]
fn 모두_장치와_같거나_재생_중인_것이_없으면_비어_있다() {
    let table = rates(&[("a", 48_000)]);
    assert_eq!(summarize(48_000, &ids(&["a"]), &table), ResampleSummary::default());
    assert_eq!(summarize(48_000, &[], &table), ResampleSummary::default());
}

#[test]
fn 장치가_바뀌면_같은_트랙도_다르게_센다() {
    // 비상 전환으로 44.1kHz 기본 장치가 되면 48kHz 음원이 변환 대상이 된다.
    let table = rates(&[("a", 48_000)]);
    assert_eq!(
        summarize(44_100, &ids(&["a"]), &table),
        ResampleSummary { source_rates: vec![48_000], track_count: 1 }
    );
}
