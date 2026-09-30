//! 방별 베이스 매니지먼트 라우팅 표.
//!
//! 각 메인 스피커는 **자기 방의 서브우퍼**로만 저역을 보낸다. 예전에는 서브가 전체에
//! 하나였고, 다른 방 스피커의 저역까지 그 서브로 모였다(방2의 저음이 테마1의 서브에서
//! 났다). 서브가 없는 방의 스피커는 풀레인지 그대로 둔다.
//!
//! route[ch] = 그 채널의 저역을 받을 서브 채널. 서브 자신, 서브 없는 방의 채널,
//! 스피커가 없는 채널은 None.

use rust_lib_atmos_mixer_pro::audio::bass_route::compute_bass_route;

const A: Option<u32> = Some(11);
const B: Option<u32> = Some(22);

#[test]
fn 한_방의_메인은_그_방_서브로_간다() {
    //            ch0    ch1(서브) ch2
    let is_sub = [false, true, false];
    let rooms = [A, A, A];
    let has = [true, true, true];
    assert_eq!(compute_bass_route(&is_sub, &rooms, &has), vec![Some(1), None, Some(1)]);
}

#[test]
fn 서브가_없는_방의_스피커는_풀레인지로_둔다() {
    // 방 A에는 서브가 있고 방 B에는 없다. B의 스피커 저역이 A의 서브로 가면 안 된다.
    let is_sub = [false, true, false];
    let rooms = [A, A, B];
    let has = [true, true, true];
    assert_eq!(compute_bass_route(&is_sub, &rooms, &has), vec![Some(1), None, None]);
}

#[test]
fn 방마다_자기_서브로_간다() {
    //            A메인  A서브  B메인  B서브
    let is_sub = [false, true, false, true];
    let rooms = [A, A, B, B];
    let has = [true, true, true, true];
    assert_eq!(
        compute_bass_route(&is_sub, &rooms, &has),
        vec![Some(1), None, Some(3), None]
    );
}

#[test]
fn 스피커가_없는_채널은_베이스_매니지먼트_대상이_아니다() {
    let is_sub = [false, true, false];
    let rooms = [A, A, None];
    let has = [true, true, false];
    assert_eq!(compute_bass_route(&is_sub, &rooms, &has), vec![Some(1), None, None]);
}

#[test]
fn 방이_지정되지_않은_예전_스피커끼리는_한_무리로_본다() {
    let is_sub = [false, true];
    let rooms = [None, None];
    let has = [true, true];
    assert_eq!(compute_bass_route(&is_sub, &rooms, &has), vec![Some(1), None]);
}
