use crate::common::config::Point3D;

pub const SPEED_OF_SOUND_M_S: f32 = 340.0;
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
