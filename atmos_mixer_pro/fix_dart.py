import re

with open("lib/src/rust/common/config.dart", "r") as f:
    code = f.read()

code = code.replace(
    "final double dispersionAngle;",
    "final double dispersionAngle;\n  final double size;"
)
code = code.replace(
    "required this.dispersionAngle,",
    "required this.dispersionAngle,\n    required this.size,"
)
code = code.replace(
    "dispersionAngle.hashCode;",
    "dispersionAngle.hashCode ^\n      size.hashCode;"
)
code = code.replace(
    "dispersionAngle == other.dispersionAngle;",
    "dispersionAngle == other.dispersionAngle &&\n          size == other.size;"
)

with open("lib/src/rust/common/config.dart", "w") as f:
    f.write(code)

with open("lib/src/rust/frb_generated.dart", "r") as f:
    code = f.read()

code = code.replace(
    "final var_dispersionAngle = dco_decode_f_32(deserializer);",
    "final var_dispersionAngle = dco_decode_f_32(deserializer);\n    final var_size = dco_decode_f_32(deserializer);"
)
code = code.replace(
    "dispersionAngle: var_dispersionAngle,",
    "dispersionAngle: var_dispersionAngle,\n      size: var_size,"
)
code = code.replace(
    "sse_encode_f_32(self.dispersionAngle, serializer);",
    "sse_encode_f_32(self.dispersionAngle, serializer);\n    sse_encode_f_32(self.size, serializer);"
)

with open("lib/src/rust/frb_generated.dart", "w") as f:
    f.write(code)

with open("lib/src/rust/frb_generated.web.dart", "r") as f:
    code = f.read()

code = code.replace(
    "final var_dispersionAngle = dco_decode_f_32(deserializer);",
    "final var_dispersionAngle = dco_decode_f_32(deserializer);\n    final var_size = dco_decode_f_32(deserializer);"
)
code = code.replace(
    "dispersionAngle: var_dispersionAngle,",
    "dispersionAngle: var_dispersionAngle,\n      size: var_size,"
)

with open("lib/src/rust/frb_generated.web.dart", "w") as f:
    f.write(code)

