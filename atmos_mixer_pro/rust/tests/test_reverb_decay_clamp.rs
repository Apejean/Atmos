//! 리버브 decay_time 클램프 회귀 테스트.
//!
//! 실기 재현(사용자 보고): 스피커 레이아웃의 리버브 랙에서 Decay Time
//! 슬라이더를 올리면(UI 표시상 "초" 단위, 최대 20까지 허용) 소리가 커지다가
//! 이후 무음이 되고, 그 뒤로 트랙을 다시 재생해도 소리가 나지 않았다.
//!
//! 원인: `engine.rs`의 SetSpatialReverb/SetChannelSpatialReverb 핸들러가
//! `mixer.reverb.decay = decay_time`로 직접 대입해서 `VirtualRoomReverb::
//! set_params()`의 `.clamp(0.0, 0.99)`를 건너뛰었다. `decay`는 FDN 피드백
//! 계수로 그대로 쓰이므로(`process_stereo`의
//! `next_inputs[i] = input + feedback * self.decay`), 1.0을 넘으면 발진해
//! 몇 콜백 안에 값이 폭주하고 Inf/NaN이 된다. 리버브는 딜레이 라인에 상태를
//! 유지하는 공유 자원이라, 한 번 NaN에 오염되면 그 뒤로 어떤 입력을 넣어도
//! (심지어 완전히 새 트랙이라도) 영구히 무음이 된다.

use rust_lib_atmos_mixer_pro::audio::reverb::VirtualRoomReverb;

/// UI가 보낼 수 있는 최댓값(슬라이더 정규화 범위: 0.2 + 19.8 = 20.0초)을
/// 그대로 흘려도 발진하지 않고, 이후 정상적인 새 입력에 유한한 값으로
/// 반응해야 한다.
#[test]
fn extreme_decay_time_does_not_blow_up_and_does_not_permanently_silence_reverb() {
    let mut reverb = VirtualRoomReverb::new(48000.0);
    // UI 슬라이더 최댓값을 그대로 흘린다 — 예전 코드라면 mixer.reverb.decay = 20.0.
    reverb.set_full_params(true, 1.0, 20.0, 0.0, 0.5, 0.5, 1.0);

    // 몇 초 분량을 임펄스+무음으로 돌려 발진 여부를 확인한다.
    let mut out_l = 0.0f32;
    let mut out_r = 0.0f32;
    for i in 0..(48000 * 3) {
        let input = if i == 0 { 1.0 } else { 0.0 };
        let (l, r) = reverb.process_stereo(input, input);
        out_l = l;
        out_r = r;
        assert!(
            l.is_finite() && r.is_finite(),
            "리버브 출력이 {}번째 샘플에서 발산했다(Inf/NaN). L={}, R={}",
            i, l, r
        );
        assert!(
            l.abs() < 10.0 && r.abs() < 10.0,
            "리버브 출력이 {}번째 샘플에서 비정상적으로 커졌다(발진 초기 증상). L={}, R={}",
            i, l, r
        );
    }

    // 마지막 상태에서도 유한해야 하고(발산 없음),
    let _ = (out_l, out_r);

    // 오염되지 않았다면 새 임펄스에 다시 정상적으로 반응해야 한다
    // (사용자가 "트랙을 다시 재생해도 소리가 안 남"이라 보고한 부분).
    let (l2, r2) = reverb.process_stereo(1.0, 1.0);
    assert!(
        l2.is_finite() && r2.is_finite(),
        "극단적 decay_time을 겪은 뒤 새 입력에 대한 응답이 발산했다(영구 무음 회귀)"
    );
}

/// decay_time이 정확히 클램프 상한(0.99)으로 잘리는지 직접 확인한다.
#[test]
fn set_full_params_clamps_decay_to_stable_range() {
    let mut reverb = VirtualRoomReverb::new(48000.0);
    reverb.set_full_params(true, 1.0, 20.0, 0.0, 0.5, 0.5, 1.0);
    assert!(
        reverb.decay <= 0.99,
        "decay가 클램프되지 않았다: {}",
        reverb.decay
    );

    reverb.set_full_params(true, 1.0, -5.0, 0.0, 0.5, 0.5, 1.0);
    assert!(
        reverb.decay >= 0.0,
        "decay 음수 입력이 클램프되지 않았다: {}",
        reverb.decay
    );
}

/// dry_wet/room_size 등 다른 파라미터도 극단값에서 안전 범위로 잘려야 한다.
#[test]
fn set_full_params_clamps_all_fields() {
    let mut reverb = VirtualRoomReverb::new(48000.0);
    reverb.set_full_params(true, 999.0, 999.0, -50.0, 999.0, 999.0, 999.0);

    assert!(reverb.room_size >= 0.1 && reverb.room_size <= 3.0);
    assert!(reverb.decay >= 0.0 && reverb.decay <= 0.99);
    assert!(reverb.pre_delay_ms >= 0.0);
    assert!(reverb.damp >= 0.0 && reverb.damp <= 1.0);
    assert!(reverb.density >= 0.0 && reverb.density <= 1.0);
    assert!(reverb.mix >= 0.0 && reverb.mix <= 1.0);
}
