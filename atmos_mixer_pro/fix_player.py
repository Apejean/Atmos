with open("rust/src/audio/player.rs", "r") as f:
    code = f.read()

code = code.replace("vec![0.0; 24]", "vec![0.0; 128]")

with open("rust/src/audio/player.rs", "w") as f:
    f.write(code)

