with open("rust/src/api/simple.rs", "r") as f:
    content = f.read()

content = content.replace("rotation_pan: 0.0,", "yaw_rotation: 0.0,")

with open("rust/src/api/simple.rs", "w") as f:
    f.write(content)
