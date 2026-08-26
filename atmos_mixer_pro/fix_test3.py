with open("rust/tests/test_bed_object_matrix.rs", "r") as f:
    code = f.read()

code = code.replace(
    "mixer.process(&mut dummy_data, out_channels);",
    "mixer.process(&mut dummy_data, out_channels);\n    println!(\"val at 0: {}\", dummy_data[0]);"
)

with open("rust/tests/test_bed_object_matrix.rs", "w") as f:
    f.write(code)

