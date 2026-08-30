import re

def main():
    path = "assets/3d_simulator/studio_engine.html"
    with open(path, "r") as f:
        content = f.read()

    content = content.replace(
        "ctx.fillText(`CH ${speakerData.channel || 1}`, 64, 32);",
        "ctx.fillText(`CH ${(speakerData.channel !== undefined ? speakerData.channel + 1 : 1)}`, 64, 32);"
    )

    with open(path, "w") as f:
        f.write(content)

main()
