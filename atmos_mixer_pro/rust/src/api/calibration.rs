use crate::common::config::Point3D;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct CalibrationResult {
    pub channel: usize,
    pub delay_ms: f32,
    pub gain_db: f32,
}

pub fn calculate_3d_calibration(
    room_width: f32,
    room_depth: f32,
    ear_level: f32,
    speakers: Vec<(usize, Point3D)>, // (channel, position)
) -> Vec<CalibrationResult> {
    if speakers.is_empty() {
        return vec![];
    }

    let ref_x = room_width / 2.0;
    let ref_y = room_depth / 2.0;
    let ref_z = ear_level;

    let mut distances = Vec::new();
    let mut max_dist = 0.0_f32;

    for (ch, pos) in &speakers {
        let dx = pos.x - ref_x;
        let dy = pos.y - ref_y;
        let dz = pos.z - ref_z;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        distances.push((*ch, dist));
        if dist > max_dist {
            max_dist = dist;
        }
    }

    let speed_of_sound = 343.0; // m/s

    let mut results = Vec::new();
    for (ch, dist) in distances {
        let delay_ms = ((max_dist - dist) / speed_of_sound) * 1000.0;
        
        let mut gain_db = 0.0;
        if dist > 0.1 && max_dist > 0.1 {
            // Attenuate closer speakers to match the furthest speaker's SPL at the listening position
            // SPL drops by 6dB per doubling of distance.
            // SPL_far = SPL_close - 20*log10(max_dist / dist)
            // So we need to apply gain_db to the closer speaker to match the far one.
            // gain_db = 20 * log10(dist / max_dist) (which will be negative)
            gain_db = 20.0 * (dist / max_dist).log10();
        }

        results.push(CalibrationResult {
            channel: ch,
            delay_ms,
            gain_db,
        });
    }

    results
}
