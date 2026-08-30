import re

def main():
    path = "assets/3d_simulator/studio_engine.html"
    with open(path, "r") as f:
        content = f.read()

    # Change dispersion cone Y position from 0.09 to 0.0
    content = content.replace(
        "dispCone.position.set(0, 0.09, frontZ);",
        "dispCone.position.set(0, 0.0, frontZ);"
    )

    with open(path, "w") as f:
        f.write(content)

main()
