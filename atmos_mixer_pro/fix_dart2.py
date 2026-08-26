with open("lib/src/rust/frb_generated.dart", "r") as f:
    code = f.read()

code = code.replace(
    "dispersionAngle: dco_decode_f_32(arr[5]),",
    "dispersionAngle: dco_decode_f_32(arr[5]),\n      size: 0.0,"
)

code = code.replace(
    "var var_dispersionAngle = sse_decode_f_32(deserializer);",
    "var var_dispersionAngle = sse_decode_f_32(deserializer);\n    var var_size = sse_decode_f_32(deserializer);"
)

with open("lib/src/rust/frb_generated.dart", "w") as f:
    f.write(code)
