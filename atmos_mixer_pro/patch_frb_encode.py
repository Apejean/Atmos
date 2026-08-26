with open("rust/src/frb_generated.rs", "r") as f:
    content = f.read()

content = content.replace(
    "        self.position.into_into_dart().into_dart(),\n        ]\n        .into_dart()",
    "        self.position.into_into_dart().into_dart(),\n            self.phase_invert.into_into_dart().into_dart(),\n            self.gain_db.into_into_dart().into_dart(),\n        ]\n        .into_dart()"
)

content = content.replace(
    "        <Option<crate::common::config::Point3D>>::sse_encode(self.position, serializer);\n    }",
    "        <Option<crate::common::config::Point3D>>::sse_encode(self.position, serializer);\n        <bool>::sse_encode(self.phase_invert, serializer);\n        <f32>::sse_encode(self.gain_db, serializer);\n    }"
)

with open("rust/src/frb_generated.rs", "w") as f:
    f.write(content)
