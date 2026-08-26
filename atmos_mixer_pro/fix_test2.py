with open("rust/tests/test_bed_object_matrix.rs", "r") as f:
    code = f.read()

code = code.replace(
    "mixer.process(&mut dummy_data, out_channels);",
    "for _ in 0..100 {\n        mixer.process(&mut dummy_data, out_channels);\n    }"
)

with open("rust/tests/test_bed_object_matrix.rs", "w") as f:
    f.write(code)

