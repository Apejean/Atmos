with open("rust/src/api/simple.rs", "r") as f:
    content = f.read()

import re

# Replace the api_calculate_3d_calibration
old_func = r"""pub fn api_calculate_3d_calibration\(
    room_width: f32,
    room_depth: f32,
    ear_level: f32,
    speaker_channels: Vec<usize>,
    speaker_x: Vec<f32>,
    speaker_y: Vec<f32>,
    speaker_z: Vec<f32>,
\) -> Vec<CalibrationResult> \{
    let mut speakers = Vec::new\(\);
    for i in 0\.\.speaker_channels\.len\(\) \{
        if i < speaker_x\.len\(\) && i < speaker_y\.len\(\) && i < speaker_z\.len\(\) \{
            speakers\.push\(\(
                speaker_channels\[i\],
                Point3D \{
                    x: speaker_x\[i\],
                    y: speaker_y\[i\],
                    z: speaker_z\[i\],
                    size: 1\.0,
                    dispersion_angle: 90\.0,
                    pitch_tilt: 0\.0,
                    yaw_rotation: 0\.0,
                \}
            \)\);
        \}
    \}
    crate::api::calibration::calculate_3d_calibration\(room_width, room_depth, ear_level, speakers\)
\}"""

new_func = """pub fn api_calculate_3d_calibration(
    room_width: f32,
    room_depth: f32,
    ear_level: f32,
    specs: Vec<crate::api::calibration::SpeakerPhysicalSpec>,
) -> Vec<CalibrationResult> {
    crate::api::calibration::calculate_3d_calibration(room_width, room_depth, ear_level, specs)
}"""

content = re.sub(old_func, new_func, content, flags=re.MULTILINE)

with open("rust/src/api/simple.rs", "w") as f:
    f.write(content)
