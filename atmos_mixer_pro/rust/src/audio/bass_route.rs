//! 방별 베이스 매니지먼트 라우팅 표. 공간 설정이 들어올 때 오디오 스레드 밖
//! (api_update_spatial_config_json)에서 계산해 엔진에 넘긴다.
//!
//! 각 메인 스피커는 **자기 방의 서브우퍼**로만 저역을 보낸다. 예전에는 서브가 전체에
//! 하나라서 다른 방 스피커의 저역까지 그 서브로 모였다(방2의 저음이 테마1의 서브에서
//! 났다). 서브가 없는 방의 스피커는 풀레인지 그대로 둔다.

/// route[ch] = 그 채널의 저역을 받을 서브 채널.
///
/// - 서브 자신, 서브가 없는 방의 채널, 스피커가 없는 채널은 None.
/// - 방이 같다는 건 방 ID가 같다는 뜻이다. 방이 지정되지 않은 예전 스피커(None)끼리는
///   한 무리로 본다.
/// - 한 방에 서브가 여럿이면 번호가 가장 작은 서브를 쓴다(화면은 방마다 1대로 제한한다).
pub fn compute_bass_route(
    is_sub: &[bool],
    room_ids: &[Option<u32>],
    has_speaker: &[bool],
) -> Vec<Option<usize>> {
    let room_of = |ch: usize| room_ids.get(ch).copied().flatten();
    let is_speaker = |ch: usize| has_speaker.get(ch).copied().unwrap_or(false);
    (0..is_sub.len())
        .map(|ch| {
            if is_sub[ch] || !is_speaker(ch) {
                return None;
            }
            let room = room_of(ch);
            (0..is_sub.len()).find(|&s| is_sub[s] && is_speaker(s) && room_of(s) == room)
        })
        .collect()
}
