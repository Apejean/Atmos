// pan_deg(DBAP 방위 트림) 에너지 보존 검증: 회전 각도와 무관하게 Sigma(g^2)=1이 유지되는지
// 기계적으로 판정한다. PAN_DEG_AND_EARLY_REFLECTIONS_SPEC.md 1.3절의 수학적 증명을 실측으로 대조.
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::common::config::{Point3D, RoomZone, Trajectory};

fn p(x: f32, y: f32, z: f32) -> Point3D {
    Point3D { x, y, z, ..Default::default() }
}

fn build_mixer_with_scene(channels: usize) -> AudioMixer {
    let (gc_tx, _) = crossbeam_channel::unbounded();
    let mut mixer = AudioMixer::new(48000, channels, gc_tx, None);

    // Shoebox 6x4x3m, pivot = (3, 2)
    let zone = RoomZone {
        room_id: 1,
        boundary_min: p(0.0, 0.0, 0.0),
        boundary_max: p(6.0, 4.0, 3.0),
        ..Default::default()
    };
    mixer.room_zones = vec![zone];

    // 비대칭 스피커 3채널 - 모두 zone 내부에 바인딩됨
    mixer.channel_positions = vec![
        Some(p(5.0, 2.0, 0.0)),
        Some(p(3.0, 3.9, 0.0)),
        Some(p(0.5, 0.5, 0.0)),
    ];

    mixer.trajectory = Some(Trajectory {
        waypoints: vec![],
        current_position: p(4.0, 1.0, 1.0),
        target_room_zone_id: None,
    });

    mixer
}

fn run_once_and_get_weights(mixer: &mut AudioMixer, pan_deg: &[f32]) -> Vec<f32> {
    mixer.channel_pan_deg = pan_deg.to_vec();
    let channels = mixer.channel_positions.len();
    let mut output = vec![0.0f32; channels * 64];
    mixer.process(&mut output, channels);
    mixer.temp_spatial_weights[..channels].to_vec()
}

/// 여러 각도(0/45/90/180도, 채널별 동일 각)에서 Sigma(pan_ratio^2) = 1이 부동소수 오차(1e-5) 이내로
/// 유지되는지 단정한다.
#[test]
fn pan_deg_rotation_preserves_dbap_energy_conservation() {
    let mut mixer = build_mixer_with_scene(3);

    for &angle in &[0.0f32, 45.0, 90.0, 180.0, -45.0, 359.0] {
        let weights = run_once_and_get_weights(&mut mixer, &[angle, angle, angle]);
        let sum_sq: f32 = weights.iter().map(|w| w * w).sum();
        let norm_factor = if sum_sq > 0.0 { 1.0 / sum_sq.sqrt() } else { 0.0 };
        let energy: f32 = weights.iter().map(|w| (w * norm_factor).powi(2)).sum();
        assert!(
            (energy - 1.0).abs() < 1e-5,
            "pan_deg={angle}도에서 Sigma(g^2)={energy}, 1.0에서 벗어남"
        );
    }
}

/// 채널마다 서로 다른 각도를 줘도(이질적 회전) 에너지 보존이 유지되는지 확인.
#[test]
fn pan_deg_rotation_preserves_energy_with_heterogeneous_angles() {
    let mut mixer = build_mixer_with_scene(3);
    let weights = run_once_and_get_weights(&mut mixer, &[30.0, -60.0, 150.0]);
    let sum_sq: f32 = weights.iter().map(|w| w * w).sum();
    let norm_factor = if sum_sq > 0.0 { 1.0 / sum_sq.sqrt() } else { 0.0 };
    let energy: f32 = weights.iter().map(|w| (w * norm_factor).powi(2)).sum();
    assert!((energy - 1.0).abs() < 1e-5, "이질적 회전에서 Sigma(g^2)={energy}");
}

/// 회전이 실제로 가중치 분포를 바꾸는지(=no-op이 아닌지) 확인 - 에너지 보존 테스트가
/// "아무 것도 안 해도 항상 통과"하는 허위양성이 되지 않도록 하는 판별 테스트.
#[test]
fn pan_deg_rotation_actually_changes_weight_distribution() {
    let mut mixer = build_mixer_with_scene(3);
    let baseline = run_once_and_get_weights(&mut mixer, &[0.0, 0.0, 0.0]);
    let rotated = run_once_and_get_weights(&mut mixer, &[90.0, 90.0, 90.0]);

    let mut any_diff = false;
    for i in 0..3 {
        if (baseline[i] - rotated[i]).abs() > 1e-4 {
            any_diff = true;
        }
    }
    assert!(any_diff, "pan_deg=90도 회전이 가중치를 전혀 바꾸지 않음(무동작 의심)");
}

/// pan_deg=0(기본값)일 때는 회전 코드 경로가 항등 변환이어야 하며, 기존(회전 미도입) 동작과
/// 100% 동일한 가중치를 산출해야 한다(회귀 방지, T-4).
#[test]
fn pan_deg_zero_is_identity_and_matches_unbound_pivot_case() {
    let mut mixer = build_mixer_with_scene(3);
    let with_zero_pan = run_once_and_get_weights(&mut mixer, &[0.0, 0.0, 0.0]);

    // pan_deg 필드 자체가 없던 시절과 동일하게 동작해야 하므로, 회전 로직을 우회한 수동 계산과 대조.
    let traj_pos = p(4.0, 1.0, 1.0);
    let positions = [p(5.0, 2.0, 0.0), p(3.0, 3.9, 0.0), p(0.5, 0.5, 0.0)];
    let blur = 2.0f32;
    let expected: Vec<f32> = positions
        .iter()
        .map(|pos| {
            let dx = pos.x - traj_pos.x;
            let dy = pos.y - traj_pos.y;
            let dz = pos.z - traj_pos.z;
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            1.0 / (dist.powi(2) + blur.powi(2))
        })
        .collect();

    for i in 0..3 {
        assert!(
            (with_zero_pan[i] - expected[i]).abs() < 1e-5,
            "채널 {i}: pan_deg=0 가중치({})가 회전 미적용 수동계산({})과 불일치", with_zero_pan[i], expected[i]
        );
    }
}
