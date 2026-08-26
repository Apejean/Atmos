with open("rust/tests/test_bed_object_matrix.rs", "r") as f:
    code = f.read()

code = code.replace("!dummy_data[1].abs() > 0.0", "dummy_data[1].abs() == 0.0")

with open("rust/tests/test_bed_object_matrix.rs", "w") as f:
    f.write(code)
