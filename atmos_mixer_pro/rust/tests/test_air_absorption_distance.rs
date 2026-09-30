//! 공기 흡음(거리에 따른 고역 감쇠)은 **스피커에서 청취 지점까지의 실제 거리**로 걸어야 한다.
//!
//! 공기 흡음은 헤드폰 미리듣기의 전파 흉내에서 건다(binaural.rs Propagation — 현장 출력에는
//! 실제 공기가 있으므로 걸지 않는다). 이 테스트는 그 거리가 맞는지 본다.
//!
//! 엔진은 이 거리를 방 중심으로부터 구하면서 축을 섞고 있었다:
//! `Point3D { x: 방 x중심, y: 스피커 y, z: 방 y중심 }` — 높이(z)를 방의 깊이(y) 중심과
//! 비교하고, 프론트엔드가 보내는 마네킹 좌표(listener_position)는 아예 쓰지 않았다.
//! 그래서 천장에 높이 매단 스피커는 실제 16m인데 8.7m로 계산돼 고역 감쇠가 절반만
//! 걸렸다(실기 증상: "고역이 세게 들어온다").

use rust_lib_atmos_mixer_pro::audio::acoustic::distance_3d;
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, Point3D, RoomZone};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;

const CH: usize = 16;

fn p(x: f32, y: f32, z: f32) -> Point3D {
    Point3D { x, y, z, ..Default::default() }
}

/// 사용자 실제 방: 15m x 20m, 천장 15m, 귀 높이 1.6m.
fn room() -> RoomZone {
    RoomZone {
        room_id: 1,
        boundary_min: p(0.0, 0.0, 0.0),
        boundary_max: p(15.0, 20.0, 15.0),
        ear_level: 1.6,
        ..Default::default()
    }
}

fn mixer_with(speaker: Point3D, listener: Option<Point3D>) -> AudioMixer {
    *GLOBAL_STATE.config.write().unwrap() = Some(AppConfig::default());
    let (gc_tx, _) = crossbeam_channel::unbounded();
    let mut mixer = AudioMixer::new(48000, CH, 512, gc_tx, None);
    mixer.room_zones = vec![room()];
    mixer.channel_positions = vec![Some(speaker)];
    mixer.channel_room_ids = vec![Some(1)];
    mixer.listener_position = listener;
    mixer.recalculate_binaural_channel_azimuths();
    mixer
}

#[test]
fn 공기_흡음은_청취_지점까지의_실제_거리로_걸린다() {
    // 천장 근처(14.75m)에 매단 스피커. 마네킹은 방 중심, 귀 높이.
    let speaker = p(14.8, 4.15, 14.75);
    let listener = p(7.5, 10.0, 1.6);
    let expected = distance_3d(&speaker, &listener);
    assert!((expected - 16.14).abs() < 0.05, "픽스처 거리 확인: {expected:.2}m");

    let mixer = mixer_with(speaker, Some(listener));
    let got = mixer.binaural.debug_propagation_distance(0);
    assert!(
        (got - expected).abs() < 0.05,
        "공기 흡음 거리가 {got:.2}m다(기대 {expected:.2}m). 높이를 방 깊이 중심과 비교하는 \
         축 혼용이 남아 있으면 8.7m 정도로 나온다"
    );
}

#[test]
fn 청취_지점이_없으면_방_중심_귀_높이를_쓴다() {
    let speaker = p(2.0, 3.0, 5.0);
    let zone = room();
    let fallback = p(
        (zone.boundary_min.x + zone.boundary_max.x) * 0.5,
        (zone.boundary_min.y + zone.boundary_max.y) * 0.5,
        zone.ear_level,
    );
    let expected = distance_3d(&speaker, &fallback);

    let mixer = mixer_with(speaker, None);
    let got = mixer.binaural.debug_propagation_distance(0);
    assert!(
        (got - expected).abs() < 0.05,
        "청취 지점 폴백 거리가 {got:.2}m다(기대 {expected:.2}m)"
    );
}

#[test]
fn 스피커가_청취_지점에_가까우면_고역을_깎지_않는다() {
    // 회귀 방지: 거리가 짧으면 감쇠가 거의 없어야 한다(축 혼용으로 엉뚱하게 멀어지면 깎였다).
    let listener = p(7.5, 10.0, 1.6);
    let speaker = p(7.5, 9.0, 1.6);
    let mixer = mixer_with(speaker, Some(listener));
    assert!(
        mixer.binaural.debug_propagation_distance(0) < 1.5,
        "가까운 스피커인데 거리가 {:.2}m로 계산됐다",
        mixer.binaural.debug_propagation_distance(0)
    );
}
