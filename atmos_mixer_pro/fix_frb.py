import re

with open("rust/src/frb_generated.rs", "r") as f:
    code = f.read()

# Add to SSE point3d decode
code = code.replace(
    "let mut var_dispersionAngle = <f32>::sse_decode(deserializer);",
    "let mut var_dispersionAngle = <f32>::sse_decode(deserializer);\n        let mut var_size = <f32>::sse_decode(deserializer);"
)
code = code.replace(
    "dispersion_angle: var_dispersionAngle,",
    "dispersion_angle: var_dispersionAngle,\n            size: var_size,"
)

# Add to SSE point3d encode
code = code.replace(
    "<f32>::sse_encode(self.dispersion_angle, serializer);",
    "<f32>::sse_encode(self.dispersion_angle, serializer);\n        <f32>::sse_encode(self.size, serializer);"
)

with open("rust/src/frb_generated.rs", "w") as f:
    f.write(code)

