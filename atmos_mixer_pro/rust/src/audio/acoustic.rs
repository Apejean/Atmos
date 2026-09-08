use crate::common::config::{Point3D, RoomZone};

pub const SPEED_OF_SOUND_M_S: f32 = 340.0;
pub const CORNER_PROXIMITY_THRESHOLD_M: f32 = 1.0; // 1 meter threshold to be considered in a "corner"

/// 채널당 초기반사음(1차 반사) 탭 고정 개수: 바닥/천장/±X벽/±Y벽 6면.
/// 가변 길이 금지(오디오 스레드가 항상 동일 인덱싱으로 읽기 위함).
pub const MAX_EARLY_REFLECTION_TAPS: usize = 6;

/// 초기반사음 딜레이 상한(ms). dsp.rs의 EARLY_REFLECTION_BUFFER_SIZE(200ms) 링버퍼 랩어라운드 방지용 방어적 클램프.
pub const EARLY_REFLECTION_MAX_DELAY_MS: f32 = 200.0;

/// Image-Source 1차 반사 탭 하나(딜레이/게인). 샘플 변환은 소비 시점(dsp.rs process())에서 fs로 수행.
#[derive(Debug, Clone, Copy, Default)]
pub struct EarlyReflectionTap {
    pub delay_ms: f32,
    pub gain: f32,
}

/// Shoebox(직육면체) 룸의 6개 평면(바닥/천장/±X벽/±Y벽)에 대해 image-source 1차 반사 탭을 계산한다.
/// 순수 함수(힙 할당 없음, 오디오 스레드 밖 - api_update_spatial_config_json에서 호출).
///
/// 탭 배열 인덱스 순서(고정): 0=바닥(z=min), 1=천장(z=max), 2=-X벽, 3=+X벽, 4=-Y벽, 5=+Y벽.
pub fn compute_early_reflection_taps(
    speaker_pos: &Point3D,
    zone: &RoomZone,
) -> [EarlyReflectionTap; MAX_EARLY_REFLECTION_TAPS] {
    let listener = Point3D {
        x: (zone.boundary_min.x + zone.boundary_max.x) * 0.5,
        y: (zone.boundary_min.y + zone.boundary_max.y) * 0.5,
        z: zone.ear_level,
        ..Default::default()
    };

    // 압력 반사계수: reflect_r = sqrt(1 - absorption_coeff)
    let reflect_r = (1.0 - zone.absorption_coeff.clamp(0.0, 0.99)).sqrt();

    // 6개 평면에 대한 image source (스피커 좌표를 해당 평면에 대해 축 하나만 대칭)
    let images: [Point3D; MAX_EARLY_REFLECTION_TAPS] = [
        Point3D { x: speaker_pos.x, y: speaker_pos.y, z: 2.0 * zone.boundary_min.z - speaker_pos.z, ..Default::default() }, // 바닥
        Point3D { x: speaker_pos.x, y: speaker_pos.y, z: 2.0 * zone.boundary_max.z - speaker_pos.z, ..Default::default() }, // 천장
        Point3D { x: 2.0 * zone.boundary_min.x - speaker_pos.x, y: speaker_pos.y, z: speaker_pos.z, ..Default::default() }, // -X벽
        Point3D { x: 2.0 * zone.boundary_max.x - speaker_pos.x, y: speaker_pos.y, z: speaker_pos.z, ..Default::default() }, // +X벽
        Point3D { x: speaker_pos.x, y: 2.0 * zone.boundary_min.y - speaker_pos.y, z: speaker_pos.z, ..Default::default() }, // -Y벽
        Point3D { x: speaker_pos.x, y: 2.0 * zone.boundary_max.y - speaker_pos.y, z: speaker_pos.z, ..Default::default() }, // +Y벽
    ];

    let mut taps = [EarlyReflectionTap::default(); MAX_EARLY_REFLECTION_TAPS];
    for i in 0..MAX_EARLY_REFLECTION_TAPS {
        let path_len = distance_3d(&images[i], &listener);
        let delay_ms = calculate_acoustic_delay_ms(path_len).min(EARLY_REFLECTION_MAX_DELAY_MS);
        let gain = reflect_r / path_len.max(1.0);
        taps[i] = EarlyReflectionTap { delay_ms, gain };
    }
    taps
}

/// Calculates 2D Euclidean distance on the X-Z plane
pub fn distance_2d(p1: &Point3D, p2: &Point3D) -> f32 {
    let dx = p1.x - p2.x;
    let dz = p1.z - p2.z;
    (dx * dx + dz * dz).sqrt()
}

/// Calculates 3D Euclidean distance
pub fn distance_3d(p1: &Point3D, p2: &Point3D) -> f32 {
    let dx = p1.x - p2.x;
    let dy = p1.y - p2.y;
    let dz = p1.z - p2.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Acoustic Delay ($t = d/v$) in milliseconds
pub fn calculate_acoustic_delay_ms(distance_meters: f32) -> f32 {
    (distance_meters / SPEED_OF_SOUND_M_S) * 1000.0
}

/// Point-in-Polygon containment via Raycasting algorithm (2D X-Z plane)
pub fn is_point_in_polygon(point: &Point3D, polygon: &[Point3D]) -> bool {
    if polygon.len() < 3 {
        return false;
    }

    let mut inside = false;
    let mut j = polygon.len() - 1;

    let x = point.x;
    let z = point.z;

    for i in 0..polygon.len() {
        let pi = &polygon[i];
        let pj = &polygon[j];

        let intersect = ((pi.z > z) != (pj.z > z))
            && (x < (pj.x - pi.x) * (z - pi.z) / (pj.z - pi.z + 1e-9) + pi.x);

        if intersect {
            inside = !inside;
        }
        j = i;
    }

    inside
}

/// Detects if a speaker point is near a polygon vertex (corner boundary loading condition)
pub fn is_in_corner(point: &Point3D, polygon: &[Point3D]) -> bool {
    for vertex in polygon {
        if distance_2d(point, vertex) < CORNER_PROXIMITY_THRESHOLD_M {
            return true;
        }
    }
    false
}

/// Calculates the forward vector from pitch (tilt) and yaw (rotation) in degrees
pub fn calculate_forward_vector(pitch_tilt: f32, yaw_rotation: f32) -> Point3D {
    let pitch_rad = pitch_tilt.to_radians();
    let yaw_rad = yaw_rotation.to_radians();
    
    // Assuming standard spherical coordinates (Y up, Z forward, X right)
    let x = pitch_rad.cos() * yaw_rad.sin();
    let y = pitch_rad.sin();
    let z = pitch_rad.cos() * yaw_rad.cos();
    
    Point3D { x, y, z, ..Default::default() }
}

/// Creates a vector from point p1 to point p2
pub fn vector_from_to(p1: &Point3D, p2: &Point3D) -> Point3D {
    Point3D {
        x: p2.x - p1.x,
        y: p2.y - p1.y,
        z: p2.z - p1.z,
        ..Default::default()
    }
}

/// Normalizes a vector. Returns (0, 0, 0) if length is 0.
pub fn normalize(v: &Point3D) -> Point3D {
    let len = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
    if len > 1e-9 {
        Point3D {
            x: v.x / len,
            y: v.y / len,
            z: v.z / len,
            ..Default::default()
        }
    } else {
        Point3D { x: 0.0, y: 0.0, z: 0.0, ..Default::default() }
    }
}

/// Calculates the angle in degrees between two normalized vectors
pub fn angle_between_vectors(v1: &Point3D, v2: &Point3D) -> f32 {
    let dot = v1.x * v2.x + v1.y * v2.y + v1.z * v2.z;
    // Clamp dot product to avoid NaN in acos due to floating point inaccuracies
    let dot_clamped = dot.clamp(-1.0, 1.0);
    dot_clamped.acos().to_degrees()
}

#[cfg(test)]
mod early_reflection_tests {
    use super::*;
    use crate::common::config::RoomZone;

    fn shoebox_zone(boundary_min: Point3D, boundary_max: Point3D, absorption_coeff: f32, ear_level: f32) -> RoomZone {
        RoomZone {
            room_id: 1,
            boundary_min,
            boundary_max,
            boundary_delay_ms: 0.0,
            boundary_eq_bands: vec![],
            transmission_loss_db: 0.0,
            absorption_coeff,
            ear_level,
        }
    }

    fn p(x: f32, y: f32, z: f32) -> Point3D {
        Point3D { x, y, z, ..Default::default() }
    }

    /// B-2/6장 검증 계획: 명세서 손계산 픽스처(방 6x4x3m, 스피커(1,1,2.5), ear_level=1.2, absorption=0.2)
    /// 대조. 6개 탭 전부를 손계산값과 ±0.01 이내로 개별 단정한다.
    #[test]
    fn shoebox_first_order_reflections_match_hand_calculation() {
        let zone = shoebox_zone(p(0.0, 0.0, 0.0), p(6.0, 4.0, 3.0), 0.2, 1.2);
        let speaker = p(1.0, 1.0, 2.5);
        let taps = compute_early_reflection_taps(&speaker, &zone);

        // (name, index, expected_delay_ms, expected_gain) - 손계산값(python으로 대조 산출, 스펙 §6 floor와 일치)
        let expected: [(&str, usize, f32, f32); 6] = [
            ("floor(z=min)", 0, 12.715274, 0.206890),
            ("ceiling(z=max)", 1, 9.434715, 0.278829),
            ("-X wall", 2, 12.715274, 0.206890),
            ("+X wall", 3, 24.018808, 0.109525),
            ("-Y wall", 4, 11.272805, 0.233364),
            ("+Y wall", 5, 16.293693, 0.161453),
        ];

        for (name, idx, exp_delay, exp_gain) in expected {
            let tap = taps[idx];
            assert!(
                (tap.delay_ms - exp_delay).abs() < 0.01,
                "{name} delay_ms mismatch: got {}, expected {} (±0.01)", tap.delay_ms, exp_delay
            );
            assert!(
                (tap.gain - exp_gain).abs() < 0.01,
                "{name} gain mismatch: got {}, expected {} (±0.01)", tap.gain, exp_gain
            );
        }
    }

    /// 명세서 §6이 요구하는 "오탐(다른 평면끼리 우연히 일치) 배제" 취지 보강 테스트.
    /// 위 손계산 픽스처는 floor와 -X벽의 값이 우연히 동일(4.323193m)해 인덱스 오배치를 잡아내지 못한다.
    /// 6개 평면의 거리값이 전부 서로 다른 비대칭 픽스처로 인덱스<->평면 매핑을 교차검증한다.
    #[test]
    fn shoebox_reflections_are_distinguishable_across_all_six_planes() {
        let zone = shoebox_zone(p(0.0, 0.0, 0.0), p(10.0, 7.0, 4.0), 0.35, 1.2);
        let speaker = p(2.0, 1.5, 3.2);
        // ear_level=1.2 이므로 실제 리스너 = (zone 중심 5, 3.5, 1.2).
        let taps = compute_early_reflection_taps(&speaker, &zone);

        let expected: [(&str, usize, f32, f32); 6] = [
            ("floor(z=min)", 0, 16.731133, 0.141727),
            ("ceiling(z=max)", 1, 14.985576, 0.158236),
            ("-X wall", 2, 22.205395, 0.106787),
            ("+X wall", 3, 39.129808, 0.060600),
            ("-Y wall", 4, 18.130629, 0.130787),
            ("+Y wall", 5, 28.515764, 0.083156),
        ];

        // 6개 delay_ms가 서로 뚜렷이(0.05ms 이상) 구분되는지 우선 확인 - 인덱스 오배치 시 즉시 실패.
        for i in 0..taps.len() {
            for j in (i + 1)..taps.len() {
                assert!(
                    (taps[i].delay_ms - taps[j].delay_ms).abs() > 0.05,
                    "tap[{i}]와 tap[{j}]의 delay_ms가 우연히 일치함 - 인덱스<->평면 매핑 오류 의심"
                );
            }
        }

        for (name, idx, exp_delay, exp_gain) in expected {
            let tap = taps[idx];
            assert!(
                (tap.delay_ms - exp_delay).abs() < 0.05,
                "{name} delay_ms mismatch: got {}, expected {}", tap.delay_ms, exp_delay
            );
            assert!(
                (tap.gain - exp_gain).abs() < 0.005,
                "{name} gain mismatch: got {}, expected {}", tap.gain, exp_gain
            );
        }
    }

    /// 물리적 타당성: 모든 1차 반사음은 직접음보다 항상 늦고(경로가 길고) 항상 작아야 한다(gain 낮음).
    #[test]
    fn reflections_are_always_later_and_quieter_than_direct_sound() {
        let zone = shoebox_zone(p(0.0, 0.0, 0.0), p(6.0, 4.0, 3.0), 0.2, 1.2);
        let speaker = p(1.0, 1.0, 2.5);
        let listener = p(
            (zone.boundary_min.x + zone.boundary_max.x) * 0.5,
            (zone.boundary_min.y + zone.boundary_max.y) * 0.5,
            zone.ear_level,
        );

        let direct_dist = distance_3d(&speaker, &listener);
        let direct_delay_ms = calculate_acoustic_delay_ms(direct_dist);
        let direct_gain = 1.0 / direct_dist.max(1.0);

        assert!((direct_dist - 2.586503).abs() < 0.001);

        let taps = compute_early_reflection_taps(&speaker, &zone);
        for (i, tap) in taps.iter().enumerate() {
            assert!(tap.delay_ms > direct_delay_ms, "tap[{i}]가 직접음보다 빠름(비물리적)");
            assert!(tap.gain < direct_gain, "tap[{i}]가 직접음보다 큼(비물리적)");
        }
    }

    /// 200ms 클램프 방어 로직: 비정상적으로 거대한 방에서도 delay_ms가 EARLY_REFLECTION_MAX_DELAY_MS를 넘지 않음.
    #[test]
    fn delay_ms_is_clamped_to_buffer_size() {
        let zone = shoebox_zone(p(0.0, 0.0, 0.0), p(500.0, 500.0, 500.0), 0.2, 1.2);
        let speaker = p(1.0, 1.0, 1.0);
        let taps = compute_early_reflection_taps(&speaker, &zone);
        for tap in taps.iter() {
            assert!(tap.delay_ms <= EARLY_REFLECTION_MAX_DELAY_MS);
        }
    }
}
