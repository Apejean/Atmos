with open("rust/tests/test_bed_object_matrix.rs", "r") as f:
    code = f.read()

code = code.replace(
    "mixer.channel_positions = positions;",
    "mixer.channel_positions = positions;\n    mixer.startup_ramp.current_gain = 1.0;"
)

with open("rust/tests/test_bed_object_matrix.rs", "w") as f:
    f.write(code)

