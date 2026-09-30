use crate::common::config::{Point3D, RoomZone};

pub const SPEED_OF_SOUND_M_S: f32 = 340.0;
pub const CORNER_PROXIMITY_THRESHOLD_M: f32 = 1.0; // 1 meter threshold to be considered in a "corner"

/// 채널당 초기반사음(1차 반사) 탭 고정 개수: 바닥/천장/±X벽/±Y벽 6면.
/// 가변 길이 금지(오디오 스레드가 항상 동일 인덱싱으로 읽기 위함).
pub const MAX_EARLY_REFLECTION_TAPS: usize = 6;

/// 초기반사음 딜레이 상한(ms). dsp.rs의 EARLY_REFLECTION_BUFFER_SIZE(200ms) 링버퍼 랩어라운드 방지용 방어적 클램프.
pub const EARLY_REFLECTION_MAX_DELAY_MS: f32 = 200.0;

/// Image-Source 1차 반사 탭 하나. 샘플 변환은 소비 시점(dsp.rs process())에서 fs로 수행.
#[derive(Debug, Clone, Copy, Default)]
pub struct EarlyReflectionTap {
    pub delay_ms: f32,
    pub gain: f32,
    /// 이 반사음에 걸 고역 감쇠(4kHz 하이셸프 게인, dB, 0 이하).
    ///
    /// 이상적인 거울면을 가정하면 반사음이 직접음의 고역을 그대로 되돌려줘서, 직접음과
    /// 겹치며 4~8kHz에 큰 피크(콤필터)를 만든다. 실제로는 ① 스피커가 벽·천장 쪽으로
    /// 고역을 약하게 내고 ② 재질의 흡음률이 고역에서 더 높고 ③ 면이 고역을 흩뿌린다.
    pub hf_shelf_db: f32,
}

/// 반사음 고역 감쇠를 적용할 셸프 주파수(Hz).
///
/// 직접음의 오프액시스 자동 EQ는 4kHz를 쓴다(position_eq.dart). 반사음은 그보다 낮은
/// 2kHz로 잡는다. ① 실제 반사음은 1~2kHz부터 이미 둔해진다(스피커 지향성이 그 대역에서
/// 좁아지고, 면의 산란도 그 위에서 커진다). ② 셸프는 꺾이는 지점에서 게인의 절반만
/// 걸리므로, 4kHz로 두면 정작 귀에 걸리는 4kHz에서 절반밖에 깎이지 않는다.
pub const EARLY_REFLECTION_HF_SHELF_HZ: f32 = 2000.0;

/// 브로드밴드 흡음률 -> 4kHz 흡음률 환산 배율.
/// 대부분의 재질은 고역 흡음이 2~3배 높다(콘크리트 0.02 -> 0.05, 석고 0.1 -> 0.25).
const HF_ABSORPTION_FACTOR: f32 = 2.5;

/// 표면 산란으로 잃는 정반사 고역(dB). 실제 면은 완전한 거울이 아니라, 요철·마감·가구
/// 때문에 고역일수록 흩뿌려진다. 그 성분은 후기 잔향(리버브)이 담당한다.
const HF_SCATTERING_LOSS_DB: f32 = -4.0;

/// 4kHz 공기 흡음(dB/m, 20도·50% 습도 기준 근사). 반사음이 직접음보다 더 지나는 거리에 곱한다.
const HF_AIR_LOSS_DB_PER_M: f32 = -0.03;

/// 콘(분산각) 밖으로 1도 벗어날 때마다 잃는 고역(dB)과 그 상한.
/// Dart의 오프액시스 자동 EQ와 같은 규약을 쓴다(position_eq.dart: 0.2dB/도, 상한 18dB).
const HF_OFF_AXIS_DB_PER_DEG: f32 = 0.2;
const HF_OFF_AXIS_MAX_DB: f32 = 18.0;
/// 고역 감쇠 총합 상한(dB).
const HF_TOTAL_MAX_DB: f32 = 18.0;

/// 연출용 초기반사 효과(스피커별 ER Mix)를 100%로 올렸을 때의 목표 에너지(직접음 대비 dB).
///
/// 물리 그대로 두면 콘크리트 홀에서는 직접음보다 커지고(정위감·명료도 저하), 큰 방에서는
/// 반대로 거의 안 들린다. 현장에서 귀로 조정하는 값이므로 방과 무관하게 슬라이더 감각을
/// 같게 만든다. -6dB면 100%에서도 직접음을 넘지 않는다.
pub const EARLY_REFLECTION_EFFECT_TARGET_DB: f32 = -6.0;

/// 방 시뮬레이션(바이노럴 미리듣기)의 목표 반사 에너지(직접음 대비 dB).
///
/// 이상적인 거울면 모델을 물리 그대로 쓰면 반사 합계가 0~+3dB까지 올라간다. 실제 홀의
/// 초기반사는 좌석에서 직접음 대비 -5~-10dB 수준이고, 무엇보다 실제 반사는 방향이 서로
/// 달라서(여기서는 전부 스피커 방향으로 합산된다) 콤 간섭이 이렇게 강하지 않다.
/// 그래서 반사 합계를 이 레벨로 맞춘다 — 공간감은 남기고 날카로움을 없앤다.
pub const EARLY_REFLECTION_ROOM_SIM_TARGET_DB: f32 = -9.0;

/// 근접면 반사를 제외하는 경로 차(m). 이보다 짧으면 0, [NEAR_SURFACE_FADE_END_M]까지 서서히 켠다.
///
/// 스피커를 벽·천장에 바짝 붙여 다는 현장이 대부분이다. 그 면의 반사는 직접음과 1ms도
/// 차이 없이 거의 같은 크기로 겹쳐 콤필터(울렁거리고 비어 있는 소리)를 만든다. 게다가
/// image-source 모델은 스피커 방향성(뒤로는 고음이 약함)과 면의 크기를 무시해서 이 근접
/// 반사를 실제보다 과장한다. 경계면 효과는 자동 EQ(position_eq.dart의 경계면 저음/바닥
/// 반사 딥)가 따로 다루므로, 여기서 넣으면 중복이다.
const NEAR_SURFACE_FADE_START_M: f32 = 0.5;
const NEAR_SURFACE_FADE_END_M: f32 = 1.5;

/// Shoebox(직육면체) 룸의 6개 평면(바닥/천장/±X벽/±Y벽)에 대해 image-source 1차 반사 탭을 계산한다.
/// 순수 함수(힙 할당 없음, 오디오 스레드 밖 - api_update_spatial_config_json에서 호출).
///
/// 지연과 크기는 **직접음 기준 상대값**이다. 합성 반사음도 스피커에서 나와 청취자까지
/// 같은 거리를 다시 지나므로, 지연은 경로 차(path - direct)이고 크기는 직접음 대비
/// 비율(direct / path)이다. 예전에는 1m 기준 절대값(지연 path/c, 크기 1/path)을 써서
/// 큰 방에서 지연이 수십 ms 길고 크기가 20dB 이상 작았다.
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

    let direct_len = distance_3d(speaker_pos, &listener);

    let mut taps = [EarlyReflectionTap::default(); MAX_EARLY_REFLECTION_TAPS];
    for i in 0..MAX_EARLY_REFLECTION_TAPS {
        let path_len = distance_3d(&images[i], &listener);
        let extra_len = (path_len - direct_len).max(0.0);
        let delay_ms =
            calculate_acoustic_delay_ms(extra_len).min(EARLY_REFLECTION_MAX_DELAY_MS);
        // 스피커에 바짝 붙은 면은 서서히 제외한다(NEAR_SURFACE_FADE_* 주석 참고).
        let near_fade = ((extra_len - NEAR_SURFACE_FADE_START_M)
            / (NEAR_SURFACE_FADE_END_M - NEAR_SURFACE_FADE_START_M))
            .clamp(0.0, 1.0);
        let gain = if path_len > 0.0 {
            (reflect_r * direct_len / path_len * near_fade).clamp(0.0, 1.0)
        } else {
            0.0
        };
        // 이 반사음이 스피커에서 출발하는 방향 = 면에 대해 청취자를 반사한 점 방향.
        let mirrored_listener = mirror_point(&listener, i, zone);
        let hf_shelf_db = reflection_hf_shelf_db(
            speaker_pos,
            &mirrored_listener,
            zone.absorption_coeff,
            extra_len,
        );
        taps[i] = EarlyReflectionTap { delay_ms, gain, hf_shelf_db };
    }
    taps
}

/// `plane_index`(0=바닥,1=천장,2=-X,3=+X,4=-Y,5=+Y) 평면에 대해 점을 대칭시킨다.
fn mirror_point(p: &Point3D, plane_index: usize, zone: &RoomZone) -> Point3D {
    let mut m = p.clone();
    match plane_index {
        0 => m.z = 2.0 * zone.boundary_min.z - p.z,
        1 => m.z = 2.0 * zone.boundary_max.z - p.z,
        2 => m.x = 2.0 * zone.boundary_min.x - p.x,
        3 => m.x = 2.0 * zone.boundary_max.x - p.x,
        4 => m.y = 2.0 * zone.boundary_min.y - p.y,
        _ => m.y = 2.0 * zone.boundary_max.y - p.y,
    }
    m
}

/// 반사음 하나에 걸 고역 감쇠(4kHz 셸프, dB).
///
/// 세 가지를 더한다.
/// 1. **스피커 방향성** — 반사음이 출발하는 방향이 분산각(콘) 밖이면 고역이 약하다.
///    수평/수직을 따로 재서 더한다(Dart position_eq.dart와 같은 규약). 수직 분산각은
///    엔진까지 전달되지 않으므로 수평 값을 함께 쓴다(기본 설정이 90x90으로 같다).
/// 2. **재질의 고역 흡음** — 흡음률은 고역에서 2~3배 높다.
/// 3. **표면 산란 + 공기 흡음** — 정반사 고역 손실과 추가 경로만큼의 공기 흡음.
fn reflection_hf_shelf_db(
    speaker: &Point3D,
    mirrored_listener: &Point3D,
    absorption_coeff: f32,
    extra_len: f32,
) -> f32 {
    // 1. 방향성
    let launch = vector_from_to(speaker, mirrored_listener);
    let horiz = (launch.x * launch.x + launch.y * launch.y).sqrt();
    let mut excess_deg = 0.0;
    let half_cone = (speaker.dispersion_angle / 2.0).max(0.0);
    if speaker.dispersion_angle > 0.0 {
        if horiz > 1e-6 {
            // yaw 0도의 정면은 레이아웃 +y다(SpeakerNode 규약).
            let yaw_rad = speaker.yaw_rotation.to_radians();
            let (fx, fy) = (yaw_rad.sin(), yaw_rad.cos());
            let cos_theta = (fx * launch.x / horiz + fy * launch.y / horiz).clamp(-1.0, 1.0);
            let theta_h = cos_theta.acos().to_degrees();
            if theta_h > half_cone {
                excess_deg += theta_h - half_cone;
            }
        }
        // 수직: 조준 상하각 vs 반사음이 출발하는 상하각(둘 다 양수 = 위).
        let launch_elev = launch.z.atan2(horiz.max(1e-6)).to_degrees();
        let theta_v = (speaker.pitch_tilt - launch_elev).abs();
        if theta_v > half_cone {
            excess_deg += theta_v - half_cone;
        }
    }
    let directivity_db = -(excess_deg * HF_OFF_AXIS_DB_PER_DEG).min(HF_OFF_AXIS_MAX_DB);

    // 2. 재질의 고역 흡음
    let broadband = absorption_coeff.clamp(0.0, 0.99);
    let hf_absorption = (broadband * HF_ABSORPTION_FACTOR).clamp(0.0, 0.95);
    let material_db = 20.0
        * ((1.0 - hf_absorption).sqrt() / (1.0 - broadband).sqrt().max(1e-6))
            .max(1e-6)
            .log10();

    // 3. 산란 + 공기 흡음
    let scattering_db = HF_SCATTERING_LOSS_DB;
    let air_db = HF_AIR_LOSS_DB_PER_M * extra_len;

    (directivity_db + material_db + scattering_db + air_db).clamp(-HF_TOTAL_MAX_DB, 0.0)
}

/// 방 시뮬레이션 배율. 연출용 정규화 배율([early_reflection_effect_scale])에
/// 목표 레벨 차이만 곱한다.
pub fn room_sim_scale_from_effect_scale(effect_scale: f32) -> f32 {
    effect_scale
        * 10.0f32.powf(
            (EARLY_REFLECTION_ROOM_SIM_TARGET_DB - EARLY_REFLECTION_EFFECT_TARGET_DB) / 20.0,
        )
}

/// 고역 셸프까지 반영한 탭 한 개의 에너지.
///
/// 정규화를 브로드밴드 게인만으로 하면, 고역 감쇠로 실제로 빠진 에너지를 모른 채
/// 계산해서 슬라이더 100%가 목표보다 2dB 정도 작아진다.
/// 셸프가 깎는 대역(2kHz 위)이 오디오 대역의 약 90%라는 근사를 쓴다.
fn tap_effective_energy(t: &EarlyReflectionTap) -> f32 {
    const HF_SHELF_BAND_FRACTION: f32 = 0.9;
    let shelf_lin = 10.0f32.powf(t.hf_shelf_db / 10.0);
    t.gain * t.gain * ((1.0 - HF_SHELF_BAND_FRACTION) + HF_SHELF_BAND_FRACTION * shelf_lin)
}

/// 연출용 ER Mix 1.0이 [EARLY_REFLECTION_EFFECT_TARGET_DB]가 되도록 하는 배율.
///
/// 탭 크기는 방 크기·재질·스피커 위치에 따라 20dB 이상 차이 난다. 슬라이더는 현장에서
/// 귀로 돌리는 값이므로, 방이 달라도 같은 감각이 되도록 전체 반사 에너지를 기준에 맞춘다.
/// 힙 할당 없음(커맨드 처리 중 호출 가능).
pub fn early_reflection_effect_scale(
    taps: &[EarlyReflectionTap; MAX_EARLY_REFLECTION_TAPS],
) -> f32 {
    let energy: f32 = taps.iter().map(tap_effective_energy).sum();
    if energy <= 0.0 {
        return 0.0;
    }
    let target_energy = 10.0f32.powf(EARLY_REFLECTION_EFFECT_TARGET_DB / 10.0);
    (target_energy / energy).sqrt()
}

/// 채널(스피커)이 속한 RoomZone을 찾는다.
///
/// Flutter는 방마다 로컬 좌표(각 방의 원점 = 0,0)를 써서 모든 RoomZone이 원점에서 겹친다.
/// 그래서 좌표만으로는 어느 방인지 알 수 없고, 스피커가 속한 방 ID(`room_id`)로 찾아야 한다.
/// 방 ID가 없는 예전 payload일 때만 좌표가 들어가는 첫 번째 방을 쓴다.
/// 방 ID가 있는데 그 방이 없으면(지워진 방) 다른 방에 붙이지 않고 None을 돌려준다.
/// 힙 할당 없음(오디오 스레드에서 호출 가능).
pub fn bind_channel_zone<'a>(
    zones: &'a [RoomZone],
    room_id: Option<u32>,
    pos: &Point3D,
) -> Option<&'a RoomZone> {
    match room_id {
        Some(id) => zones.iter().find(|z| z.room_id == id),
        None => zones.iter().find(|z| {
            pos.x >= z.boundary_min.x
                && pos.x <= z.boundary_max.x
                && pos.y >= z.boundary_min.y
                && pos.y <= z.boundary_max.y
        }),
    }
}

/// 채널이 겨냥하는 청취 지점.
///
/// 우선순위: ① 프론트엔드가 보낸 마네킹 좌표(실제로 소리를 듣는 지점) ② 바인딩된 방의
/// 수평 중심 + 귀 높이 ③ 스피커 자신(거리 0 = 보정 없음).
///
/// 예전에는 `{x: 방 x중심, y: 스피커 y, z: 방 y중심}`을 썼다. 높이(z)를 방의 깊이(y)
/// 중심과 비교하는 축 혼용이라, 천장에 높이 매단 스피커의 거리가 실제의 절반으로
/// 나왔다(공기 흡음이 덜 걸려 고역이 세게 들렸다).
pub fn listening_point(
    listener: Option<&Point3D>,
    zone: Option<&RoomZone>,
    speaker: &Point3D,
) -> Point3D {
    if let Some(l) = listener {
        return Point3D { x: l.x, y: l.y, z: l.z, ..Default::default() };
    }
    if let Some(z) = zone {
        return Point3D {
            x: (z.boundary_min.x + z.boundary_max.x) * 0.5,
            y: (z.boundary_min.y + z.boundary_max.y) * 0.5,
            z: z.ear_level,
            ..Default::default()
        };
    }
    speaker.clone()
}

/// 자동 게인(거리 보정)의 기준 거리(m) = 청취 지점(방 중앙)에서 가장 가까운 벽까지, 최소 0.5m.
///
/// **Dart acoustic_sync_provider.dart의 `gainRefDistance`와 같은 식**이어야 한다. 헤드폰
/// 미리듣기의 거리 감쇠(기준/거리)와 자동 게인(20·log10(거리/기준))이 그래야 정확히 상쇄된다.
/// 방이 없으면 Dart는 청사진 캔버스를 방으로 쓰고 청취 지점을 그 중앙에 보내므로, 청취 지점
/// 좌표(= 캔버스 가로·세로의 절반)로 같은 값을 만든다. 둘 다 없으면 None.
pub fn gain_reference_distance(zone: Option<&RoomZone>, listener: Option<&Point3D>) -> Option<f32> {
    let half_min = match (zone, listener) {
        (Some(z), _) => {
            let w = (z.boundary_max.x - z.boundary_min.x).abs();
            let d = (z.boundary_max.y - z.boundary_min.y).abs();
            w.min(d) / 2.0
        }
        (None, Some(l)) => l.x.min(l.y),
        (None, None) => return None,
    };
    Some(half_min.max(0.5))
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

    /// 손계산 픽스처(방 6x4x3m, 스피커(1,1,2.5), ear_level=1.2, absorption=0.2) 대조.
    /// 6개 탭 전부를 손계산값과 ±0.01 이내로 개별 단정한다.
    ///
    /// 값은 **직접음 대비 상대값**이다: 지연 = (반사 경로 - 직접 경로)/음속,
    /// 크기 = sqrt(1-흡음) x 직접 경로/반사 경로. 천장 탭은 경로 차가 0.62m라
    /// 근접면 페이드(0.5~1.5m)가 부분 적용되어 작다.
    #[test]
    fn shoebox_first_order_reflections_match_hand_calculation() {
        let zone = shoebox_zone(p(0.0, 0.0, 0.0), p(6.0, 4.0, 3.0), 0.2, 1.2);
        let speaker = p(1.0, 1.0, 2.5);
        let taps = compute_early_reflection_taps(&speaker, &zone);

        // (name, index, expected_delay_ms, expected_gain) - 손계산값(python으로 대조 산출, 스펙 §6 floor와 일치)
        let expected: [(&str, usize, f32, f32); 6] = [
            ("floor(z=min)", 0, 5.107911, 0.535123),
            ("ceiling(z=max)", 1, 1.827352, 0.087480),
            ("-X wall", 2, 5.107911, 0.535123),
            ("+X wall", 3, 16.411444, 0.283288),
            ("-Y wall", 4, 3.665442, 0.450434),
            ("+Y wall", 5, 8.686330, 0.417599),
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
            ("floor(z=min)", 0, 4.604351, 0.584355),
            ("ceiling(z=max)", 1, 2.858794, 0.307937),
            ("-X wall", 2, 10.078614, 0.440295),
            ("+X wall", 3, 27.003027, 0.249859),
            ("-Y wall", 4, 6.003848, 0.539249),
            ("+Y wall", 5, 16.388983, 0.342860),
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

    /// 물리적 타당성: 모든 1차 반사음은 직접음보다 늦게 도착하고 더 작아야 한다.
    ///
    /// 탭 값이 직접음 **대비 상대값**이므로, 상대 지연 > 0이고 상대 크기 < 1이면 된다
    /// (직접음은 지연 0, 크기 1에 해당한다). 근접면 페이드로 제외된 탭(gain 0)은 건너뛴다.
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
        assert!((direct_dist - 2.586503).abs() < 0.001);

        let taps = compute_early_reflection_taps(&speaker, &zone);
        let mut audible = 0;
        for (i, tap) in taps.iter().enumerate() {
            if tap.gain == 0.0 {
                continue; // 스피커에 바짝 붙은 면(제외됨)
            }
            audible += 1;
            assert!(tap.delay_ms > 0.0, "tap[{i}]가 직접음보다 빠름(비물리적)");
            assert!(tap.gain < 1.0, "tap[{i}]가 직접음보다 큼(비물리적)");
        }
        assert!(audible >= 4, "이 배치에서는 반사가 대부분 남아 있어야 한다: {audible}개");
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
