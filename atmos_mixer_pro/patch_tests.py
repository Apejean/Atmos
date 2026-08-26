import re

# Fix stress_test.rs
with open("rust/tests/stress_test.rs", "r") as f:
    stress = f.read()
stress = re.sub(r"\s*stream_receiver: None,", "", stress)
with open("rust/tests/stress_test.rs", "w") as f:
    f.write(stress)

# Fix test_zero_defect_e2e.rs
with open("rust/tests/test_zero_defect_e2e.rs", "r") as f:
    zero = f.read()
zero = zero.replace("dbap.calculate_gains(0.0, 0.0, 0.0, &mut gains);", "dbap.calculate_gains(0.0, 0.0, 0.0, 0.0, &mut gains);")
with open("rust/tests/test_zero_defect_e2e.rs", "w") as f:
    f.write(zero)

# Fix test_audio_precision.rs
with open("rust/tests/test_audio_precision.rs", "r") as f:
    precision = f.read()
precision = precision.replace("spatializer.process_sample(sine_wave[100], (0.0, 0.0, 0.0), &mut out_buffer);", "spatializer.process_sample(sine_wave[100], (0.0, 0.0, 0.0), 0.0, &mut out_buffer);")
with open("rust/tests/test_audio_precision.rs", "w") as f:
    f.write(precision)
