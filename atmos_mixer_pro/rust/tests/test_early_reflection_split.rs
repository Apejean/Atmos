//! 초기반사음은 목적이 둘로 나뉜다.
//!
//! 1. **방 시뮬레이션** — 시공 전 헤드폰(바이노럴)으로 현장을 미리 들을 때, 그 방이
//!    만들 반사를 합성해서 들려준다. 직접음 대비 실제 지연과 크기여야 한다.
//! 2. **연출용 효과(스피커별 ER Mix)** — 현장에서 실제 스피커로 틀 때 귀로 조정하는
//!    값이다. 어느 방이든 슬라이더 감각이 같아야 하므로 레벨을 정규화한다.
//!
//! 현장에서는 진짜 벽이 반사를 만들기 때문에 방 시뮬레이션을 그대로 틀면 반사가 두 번
//! 들어간다. 그래서 시뮬레이션은 바이노럴을 켤 때만 걸고, 현장(바이노럴 끔)에서는
//! 연출용 값만 적용한다.

use rust_lib_atmos_mixer_pro::audio::acoustic::{
    compute_early_reflection_taps, distance_3d, early_reflection_effect_scale,
    EARLY_REFLECTION_EFFECT_TARGET_DB, SPEED_OF_SOUND_M_S,
};
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::common::config::{Point3D, RoomZone};

const FS: f32 = 48000.0;
const CH: usize = 16;

fn p(x: f32, y: f32, z: f32) -> Point3D {
    Point3D { x, y, z, ..Default::default() }
}

/// 20m x 15m x 8m 콘크리트 홀. 귀 높이 1.6m, 흡음 0.02.
fn hall() -> RoomZone {
    RoomZone {
        boundary_min: p(0.0, 0.0, 0.0),
        boundary_max: p(20.0, 15.0, 8.0),
        absorption_coeff: 0.02,
        ear_level: 1.6,
        ..Default::default()
    }
}

fn listener(zone: &RoomZone) -> Point3D {
    p(
        (zone.boundary_min.x + zone.boundary_max.x) * 0.5,
        (zone.boundary_min.y + zone.boundary_max.y) * 0.5,
        zone.ear_level,
    )
}

#[test]
fn 반사음은_직접음_대비_상대_지연과_크기로_계산된다() {
    let zone = hall();
    let spk = p(3.0, 3.0, 5.0);
    let taps = compute_early_reflection_taps(&spk, &zone);
    let ear = listener(&zone);
    let direct = distance_3d(&spk, &ear);
    let reflect_r = (1.0f32 - zone.absorption_coeff).sqrt();

    // 0번 탭 = 바닥(z = boundary_min.z에 대한 거울상)
    let floor_image = p(spk.x, spk.y, 2.0 * zone.boundary_min.z - spk.z);
    let path = distance_3d(&floor_image, &ear);
    let extra = path - direct;
    assert!(extra > 1.5, "이 배치는 근접면 페이드 구간 밖이어야 한다: {extra:.2}m");

    let expected_delay = extra / SPEED_OF_SOUND_M_S * 1000.0;
    let expected_gain = reflect_r * direct / path;
    assert!(
        (taps[0].delay_ms - expected_delay).abs() < 0.01,
        "바닥 반사 지연: {:.2}ms (기대 {:.2}ms) — 합성 반사음도 스피커에서 나와 청취자까지 \
         다시 이동하므로, 직접음과의 **차이**만 지연으로 걸어야 한다",
        taps[0].delay_ms,
        expected_delay
    );
    assert!(
        (taps[0].gain - expected_gain).abs() < 0.001,
        "바닥 반사 크기: {:.4} (기대 {:.4}) — 직접음도 같은 거리를 지나므로 크기는 \
         직접음 대비 비율이어야 한다",
        taps[0].gain,
        expected_gain
    );
    // 반사는 직접음보다 늦고 작다(물리적 타당성).
    for t in taps.iter().filter(|t| t.gain > 0.0) {
        assert!(t.delay_ms > 0.0, "반사가 직접음보다 먼저 도착한다");
        assert!(t.gain <= 1.0, "반사가 직접음보다 크다: {}", t.gain);
    }
}

#[test]
fn 스피커에_바짝_붙은_면의_반사는_제외된다() {
    // 천장(8m) 바로 아래 0.25m에 매단 스피커. 천장 반사는 0.5m도 안 되는 경로 차이로
    // 직접음과 거의 같은 크기로 겹쳐 콤필터(울렁거림)를 만든다. 실제 현장에서는 그
    // 근접면 효과를 자동 EQ(경계면 보정)가 따로 다루므로 여기서 넣으면 중복이다.
    let zone = hall();
    let spk = p(10.0, 3.0, 7.75);
    let taps = compute_early_reflection_taps(&spk, &zone);

    assert_eq!(taps[1].gain, 0.0, "천장 반사(경로 차 0.5m 미만)가 제외되지 않았다");
    assert!(taps[0].gain > 0.0, "먼 바닥 반사는 남아 있어야 한다");
}

#[test]
fn 근접면은_갑자기_켜지지_않고_서서히_켜진다() {
    // 경로 차가 0.5m -> 1.5m 구간에서 서서히 켠다. 스피커를 끌 때 반사가 툭 생기지
    // 않게 하기 위함이다(위치는 드래그로 연속 변한다).
    let zone = hall();
    let ear = listener(&zone);
    let reflect_r = (1.0f32 - zone.absorption_coeff).sqrt();

    // 천장까지의 거리를 조금씩 늘려 경로 차가 딱 1.0m가 되는 높이를 찾는다.
    let mut found = None;
    for step in 0..4000 {
        let z = 8.0 - 0.3 - step as f32 * 0.001;
        let spk = p(10.0, 3.0, z);
        let ceiling_image = p(spk.x, spk.y, 2.0 * zone.boundary_max.z - spk.z);
        let extra = distance_3d(&ceiling_image, &ear) - distance_3d(&spk, &ear);
        if (extra - 1.0).abs() < 0.005 {
            let path = distance_3d(&ceiling_image, &ear);
            let direct = distance_3d(&spk, &ear);
            found = Some((spk, extra, path, direct));
            break;
        }
    }
    let (spk, extra, path, direct) = found.expect("경로 차 1.0m 배치를 찾지 못했다");
    let taps = compute_early_reflection_taps(&spk, &zone);
    let full = reflect_r * direct / path;
    let expected = full * 0.5; // (1.0 - 0.5) / 1.0 = 0.5

    assert!(
        (taps[1].gain - expected).abs() < 0.01 * full,
        "경로 차 {extra:.2}m에서 천장 반사가 절반으로 페이드되지 않았다: {:.4} (기대 {:.4})",
        taps[1].gain,
        expected
    );
}

/// 채널 0에 탭을 실어둔 믹서를 만든다.
fn mixer_with_taps(er_mix: f32, binaural: bool) -> AudioMixer {
    let (gc_tx, _) = crossbeam_channel::unbounded();
    let mut mixer = AudioMixer::new(48000, CH, 512, gc_tx, None);
    let zone = hall();
    let spk = p(3.0, 3.0, 5.0);
    mixer.room_zones = vec![zone.clone()];
    mixer.channel_positions = vec![Some(spk.clone())];
    let taps = compute_early_reflection_taps(&spk, &zone);
    mixer.channel_room_ids = vec![None];
    mixer.binaural.enabled = binaural;
    mixer.apply_early_reflection_taps(0, &taps);
    mixer.set_channel_early_ref_mix(0, er_mix);
    mixer
}

/// 탭 지연·크기와 믹스를 정상 상태로 맞춘다.
///
/// 실제 엔진은 이 값들을 샘플 단위로 서서히 옮긴다(Law 3). 임펄스 응답으로 레벨을 재려면
/// 그 과도 구간이 아니라 도달한 상태에서 재야 한다 — 그러지 않으면 짧은 지연 탭이 아직
/// 덜 열린 상태로 측정된다.
/// (고역 셸프 계수는 여기서 맞출 수 없다 — 값이 바뀔 때만 갱신되므로 무음을 흘려 수렴시킨다.)
fn settle_early_reflections(mixer: &mut AudioMixer, channel: usize) {
    for i in 0..6 {
        let target_delay = mixer.channel_dsp[channel].taps[i].target_delay_ms;
        let target_gain = mixer.channel_dsp[channel].taps[i].target_gain;
        mixer.channel_dsp[channel].taps[i].current_delay_ms = target_delay;
        mixer.channel_dsp[channel].taps[i].current_gain = target_gain;
    }
    mixer.channel_dsp[channel].current_early_ref_mix =
        mixer.channel_dsp[channel].target_early_ref_mix;
}

#[test]
fn 방_시뮬레이션은_바이노럴을_켤_때만_걸린다() {
    // 현장(바이노럴 끔)에서는 진짜 벽이 반사를 만든다. 합성 반사를 겹치면 안 된다.
    let field = mixer_with_taps(0.0, false);
    assert_eq!(
        field.channel_dsp[0].target_early_ref_mix, 0.0,
        "현장에서 합성 반사가 걸렸다(실제 방 반사와 이중 적용)"
    );

    // 설계(바이노럴 켬)에서는 그 방의 반사를 들려준다(레벨은 정규화된다 —
    // acoustic::EARLY_REFLECTION_ROOM_SIM_TARGET_DB).
    let preview = mixer_with_taps(0.0, true);
    assert!(
        preview.channel_dsp[0].target_early_ref_mix > 0.0,
        "헤드폰 미리듣기에 방 시뮬레이션이 걸리지 않았다"
    );
}

#[test]
fn er_믹스_100퍼센트는_직접음_대비_6db로_정규화된다() {
    // 어느 방에서든 슬라이더 감각이 같아야 현장에서 귀로 조정할 수 있다.
    let mut m = mixer_with_taps(1.0, false);
    settle_early_reflections(&mut m, 0);
    // 고역 셸프 계수는 샘플이 흐르면서 목표로 수렴한다(Law 3). 무음으로 예열한 뒤 잰다.
    for _ in 0..4800 {
        m.channel_dsp[0].process(0.0, FS);
    }

    // 임펄스를 넣어 직접음 대비 반사음 에너지를 잰다.
    let out: Vec<f32> = (0..(0.5 * FS) as usize)
        .map(|n| m.channel_dsp[0].process(if n == 0 { 1.0 } else { 0.0 }, FS))
        .collect();
    let direct = out[0] * out[0];
    let er: f32 = out[1..].iter().map(|v| v * v).sum();
    let db = 10.0 * (er / direct).log10();

    assert!(
        (db - EARLY_REFLECTION_EFFECT_TARGET_DB).abs() < 0.5,
        "ER 100%의 반사 에너지가 {db:.2}dB다(기대 {EARLY_REFLECTION_EFFECT_TARGET_DB}dB)"
    );
}

#[test]
fn 방_시뮬레이션의_반사량은_정해진_레벨로_맞춰진다() {
    use rust_lib_atmos_mixer_pro::audio::acoustic::EARLY_REFLECTION_ROOM_SIM_TARGET_DB;
    let mut m = mixer_with_taps(0.0, true); // 연출용 0%, 바이노럴 켬
    settle_early_reflections(&mut m, 0);
    for _ in 0..4800 {
        m.channel_dsp[0].process(0.0, FS);
    }
    let out: Vec<f32> = (0..(0.5 * FS) as usize)
        .map(|n| m.channel_dsp[0].process(if n == 0 { 1.0 } else { 0.0 }, FS))
        .collect();
    let direct = out[0] * out[0];
    let er: f32 = out[1..].iter().map(|v| v * v).sum();
    let db = 10.0 * (er / direct).log10();

    assert!(
        (db - EARLY_REFLECTION_ROOM_SIM_TARGET_DB).abs() < 0.5,
        "방 시뮬레이션 반사 에너지가 {db:.2}dB다(기대 {EARLY_REFLECTION_ROOM_SIM_TARGET_DB}dB)"
    );
}

#[test]
fn er_믹스가_0이면_현장_출력이_그대로다() {
    let mut with_taps = mixer_with_taps(0.0, false);
    settle_early_reflections(&mut with_taps, 0);
    let (gc_tx, _) = crossbeam_channel::unbounded();
    let mut clean = AudioMixer::new(48000, CH, 512, gc_tx, None);

    for n in 0..2000 {
        let x = if n == 0 { 1.0 } else { 0.0 };
        let a = with_taps.channel_dsp[0].process(x, FS);
        let b = clean.channel_dsp[0].process(x, FS);
        assert!((a - b).abs() < 1e-9, "{n}번째 샘플이 다르다: {a} vs {b}");
    }
}

#[test]
fn 정규화_계수는_탭이_없으면_0이다() {
    let no_taps = [Default::default(); 6];
    assert_eq!(early_reflection_effect_scale(&no_taps), 0.0);
}
