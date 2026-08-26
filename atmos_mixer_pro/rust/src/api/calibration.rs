use crate::common::config::{EqBand, EqType, Point3D};

#[derive(Clone, Debug, Default)]
pub struct SpeakerPhysicalSpec {
    pub channel: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub internal_latency_ms: f32,
    pub low_cut_hz: f32,
    pub boundary_type: String, // "Corner", "Wall", "FreeSpace"
}

#[derive(Clone, Debug, Default)]
pub struct CalibrationResult {
    pub channel: u32,
    pub delay_ms: f32,
    pub gain_db: f32,
    pub phase_invert: bool,
    pub eq_bands: Vec<EqBand>,
}

pub fn calculate_3d_calibration(
    room_width: f32,
    room_depth: f32,
    ear_level: f32,
    speakers: Vec<SpeakerPhysicalSpec>,
) -> Vec<CalibrationResult> {
    if speakers.is_empty() {
        return vec![];
    }

    let ref_x = room_width / 2.0;
    let ref_y = room_depth / 2.0;
    let ref_z = ear_level;

    let mut distances = Vec::new();
    let mut max_dist = 0.0_f32;

    for spec in &speakers {
        let dx = spec.x - ref_x;
        let dy = spec.y - ref_y;
        let dz = spec.z - ref_z;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        distances.push((spec, dist));
        if dist > max_dist {
            max_dist = dist;
        }
    }

    let speed_of_sound = 343.2; // m/s

    let mut results = Vec::new();
    for (spec, dist) in distances {
        // 1. Precise Delay Calculation
        let delay_ms = ((max_dist - dist) / speed_of_sound * 1000.0) + spec.internal_latency_ms;
        
        // 2. Gain Calculation (Inverse Square Law)
        let mut gain_db = 0.0;
        if dist > 0.1 && max_dist > 0.1 {
            gain_db = 20.0 * (dist / max_dist).log10();
        }

        // 3. Phase Auto Alignment
        let delay_diff_s = (max_dist - dist) / speed_of_sound;
        let mut phase_invert = false;
        if spec.low_cut_hz > 0.0 {
            let phase_diff_deg = (delay_diff_s * spec.low_cut_hz * 360.0) % 360.0;
            // Normalize to positive
            let phase_diff_deg = (phase_diff_deg + 360.0) % 360.0;
            if phase_diff_deg >= 150.0 && phase_diff_deg <= 210.0 {
                phase_invert = true;
            }
        }

        // 4. Boundary Bass Boom Inverse EQ
        let mut eq_bands = Vec::new();
        let bass_gain = match spec.boundary_type.as_str() {
            "Corner" => -9.0,
            "Wall" => -6.0,
            _ => 0.0, // "FreeSpace" or others
        };

        if bass_gain < 0.0 {
            eq_bands.push(EqBand {
                enabled: true,
                freq: 100.0,
                gain: bass_gain,
                q_factor: 0.707,
                filter_type: EqType::LowShelf,
            });
        }

        results.push(CalibrationResult {
            channel: spec.channel,
            delay_ms,
            gain_db,
            phase_invert,
            eq_bands,
        });
    }

    results
}
