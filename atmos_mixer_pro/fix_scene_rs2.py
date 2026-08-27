with open("rust/src/api/scene.rs", "r") as f:
    content = f.read()

content = content.replace('return Err(AtmosError { message: "Scene file not found".to_string( }));', 'return Err(AtmosError { message: "Scene file not found".to_string() });')

with open("rust/src/api/scene.rs", "w") as f:
    f.write(content)

