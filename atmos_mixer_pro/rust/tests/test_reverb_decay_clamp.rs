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

// ---------------------------------------------------------------------------
// UI 물리 단위 -> DSP 계수 변환 검증
//
// 사용자 보고: "디케이랑 프리딜레이든 여러 사이즈 바꿨는데 실시간이 안 되는
// 것 같다." 원인은 UI 슬라이더 범위와 Rust가 쓰던 계수 범위가 달라서 대부분의
// 슬라이더가 최댓값에 붙박이로 고정된 것이었다(예: roomSize는 m³ 50~2000인데
// Rust는 0.1~3.0으로 clamp -> 항상 3.0). 아래 테스트들은 각 파라미터가 UI
// 범위 전체에 걸쳐 실제로 서로 다른 값을 만들어내는지 확인한다.
// ---------------------------------------------------------------------------

/// 방 크기(m³)가 커지면 딜레이 길이 배율도 단조 증가해야 한다.
#[test]
fn room_size_m3_maps_monotonically_across_ui_range() {
    let mut r = VirtualRoomReverb::new(48000.0);

    let mut prev = f32::MIN;
    let mut seen = Vec::new();
    for m3 in [50.0f32, 200.0, 800.0, 1400.0, 2000.0] {
        r.set_full_params(true, m3, 3.2, 25.0, 50.0, 50.0, 1.0);
        let v = r.room_size;
        assert!(
            v > prev,
            "방 크기 {}m³에서 배율이 증가하지 않았다: {} -> {}",
            m3, prev, v
        );
        prev = v;
        seen.push(v);
    }

    // 기본값 800m³가 기준 배율 1.0 근처여야 한다.
    assert!(
        (seen[2] - 1.0).abs() < 0.01,
        "기본 800m³의 배율이 1.0이 아니다: {}",
        seen[2]
    );
    // 전체 범위가 한 점에 뭉개지면 안 된다(예전 버그: 전부 3.0).
    assert!(
        seen.last().unwrap() - seen.first().unwrap() > 0.5,
        "방 크기 슬라이더 전체 범위가 사실상 한 값으로 뭉개졌다: {:?}",
        seen
    );
}

/// 감쇠 시간(초)이 길어지면 피드백 계수도 커져야 하고, 항상 안정 범위여야 한다.
#[test]
fn decay_time_seconds_maps_monotonically_and_stays_stable() {
    let mut r = VirtualRoomReverb::new(48000.0);

    let mut prev = f32::MIN;
    let mut seen = Vec::new();
    for sec in [0.2f32, 1.0, 3.2, 10.0, 20.0] {
        r.set_full_params(true, 800.0, sec, 25.0, 50.0, 50.0, 1.0);
        let g = r.decay;
        assert!(
            g > prev,
            "감쇠 {}초에서 피드백 계수가 증가하지 않았다: {} -> {}",
            sec, prev, g
        );
        assert!(
            (0.0..1.0).contains(&g),
            "피드백 계수가 불안정 범위다({}초 -> {})",
            sec, g
        );
        prev = g;
        seen.push(g);
    }

    // 예전 버그: 1초만 넘으면 전부 0.99로 뭉개졌다.
    assert!(
        seen.last().unwrap() - seen.first().unwrap() > 0.3,
        "감쇠 슬라이더 전체 범위가 사실상 한 값으로 뭉개졌다: {:?}",
        seen
    );
}

/// 댐핑/밀도는 %(0~100)로 들어오므로 0.0~1.0으로 환산되어야 한다.
#[test]
fn damp_and_density_convert_from_percent() {
    let mut r = VirtualRoomReverb::new(48000.0);

    r.set_full_params(true, 800.0, 3.2, 25.0, 0.0, 0.0, 1.0);
    assert!(r.damp.abs() < 1e-6, "damp 0% -> {}", r.damp);
    assert!(r.density.abs() < 1e-6, "density 0% -> {}", r.density);

    r.set_full_params(true, 800.0, 3.2, 25.0, 50.0, 50.0, 1.0);
    assert!((r.damp - 0.5).abs() < 1e-6, "damp 50% -> {}", r.damp);
    assert!((r.density - 0.5).abs() < 1e-6, "density 50% -> {}", r.density);

    r.set_full_params(true, 800.0, 3.2, 25.0, 100.0, 100.0, 1.0);
    assert!((r.damp - 1.0).abs() < 1e-6, "damp 100% -> {}", r.damp);
    assert!((r.density - 1.0).abs() < 1e-6, "density 100% -> {}", r.density);
}

/// 프리딜레이(ms)는 그대로 쓰이되 딜레이 라인 용량 안에서만 허용되어야 한다.
#[test]
fn pre_delay_ms_is_preserved_within_ui_range() {
    let mut r = VirtualRoomReverb::new(48000.0);
    for ms in [0.5f32, 25.0, 100.0] {
        r.set_full_params(true, 800.0, 3.2, ms, 50.0, 50.0, 1.0);
        assert!(
            (r.pre_delay_ms - ms).abs() < 1e-6,
            "프리딜레이 {}ms가 보존되지 않았다: {}",
            ms, r.pre_delay_ms
        );
    }
}

/// 감쇠 시간을 바꾸면 실제 임펄스 응답의 꼬리 길이가 실제로 달라져야 한다
/// (계수만 바뀌고 소리는 그대로인 상황을 막는다).
#[test]
fn longer_decay_time_produces_longer_audible_tail() {
    fn tail_energy(decay_sec: f32) -> f32 {
        let mut r = VirtualRoomReverb::new(48000.0);
        r.set_full_params(true, 800.0, decay_sec, 0.0, 0.0, 50.0, 1.0);
        // 임펄스 후 2초 지점의 잔향 에너지를 잰다.
        let mut energy = 0.0f32;
        for i in 0..(48000 * 3) {
            let input = if i == 0 { 1.0 } else { 0.0 };
            let (l, _r) = r.process_stereo(input, input);
            if i > 48000 * 2 {
                energy += l * l;
            }
        }
        energy
    }

    let short_tail = tail_energy(0.5);
    let long_tail = tail_energy(15.0);

    assert!(
        long_tail > short_tail * 10.0,
        "감쇠 시간을 0.5초에서 15초로 늘렸는데 2초 시점 잔향 에너지가 거의 같다 \
         (슬라이더가 소리에 반영되지 않음). short={:e} long={:e}",
        short_tail, long_tail
    );
}
