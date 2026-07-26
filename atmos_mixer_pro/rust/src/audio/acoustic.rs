use crate::common::config::Point3D;

pub const SPEED_OF_SOUND_M_S: f32 = 343.0;
pub const CORNER_PROXIMITY_THRESHOLD_M: f32 = 1.0; // 1 meter threshold to be considered in a "corner"

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
