import re

with open('rust/src/audio/dsp.rs', 'r') as f:
    content = f.read()

# Add target_gain_linear and current_gain_linear to ChannelDspState
struct_def = """        pub air_absorption: crate::audio::dsp::acoustic_physics::AirAbsorptionFilter,
        pub phase_invert: bool,
        pub gain_db: f32,"""

new_struct_def = """        pub air_absorption: crate::audio::dsp::acoustic_physics::AirAbsorptionFilter,
        pub phase_invert: bool,
        pub target_gain_linear: f32,
        pub current_gain_linear: f32,"""

if struct_def in content:
    content = content.replace(struct_def, new_struct_def)

default_impl = """                air_absorption: crate::audio::dsp::acoustic_physics::AirAbsorptionFilter::new(),
                phase_invert: false,
                gain_db: 0.0,"""

new_default_impl = """                air_absorption: crate::audio::dsp::acoustic_physics::AirAbsorptionFilter::new(),
                phase_invert: false,
                target_gain_linear: 1.0,
                current_gain_linear: 1.0,"""

if default_impl in content:
    content = content.replace(default_impl, new_default_impl)

# Change process loop
process_logic = """            // Apply Trim Gain
            if self.gain_db.abs() > 0.01 {
                let gain_linear = 10.0_f32.powf(self.gain_db / 20.0);
                out *= gain_linear;
            }

            // Apply Phase Invert
            if self.phase_invert {
                out *= -1.0;
            }"""

new_process_logic = """            // Smooth gain (Zipper noise prevention)
            if (self.current_gain_linear - self.target_gain_linear).abs() > 0.0001 {
                self.current_gain_linear += (self.target_gain_linear - self.current_gain_linear) * 0.002; // Simple one-pole smoothing
            } else {
                self.current_gain_linear = self.target_gain_linear;
            }
            
            out *= self.current_gain_linear;

            // Apply Phase Invert
            if self.phase_invert {
                out *= -1.0;
            }"""

if process_logic in content:
    content = content.replace(process_logic, new_process_logic)

# Add set_gain method
method_impl = """    impl ChannelDspState {"""
new_method_impl = """    impl ChannelDspState {
        pub fn set_gain_db(&mut self, db: f32) {
            self.target_gain_linear = 10.0_f32.powf(db / 20.0);
        }"""

if method_impl in content:
    content = content.replace(method_impl, new_method_impl, 1)

with open('rust/src/audio/dsp.rs', 'w') as f:
    f.write(content)

print("dsp.rs patched")
