import re

def main():
    path = "assets/3d_simulator/studio_engine.html"
    with open(path, "r") as f:
        content = f.read()

    # The error "updateScene is not defined!" happened when calling it from Dart.
    # Check if window.updateScene is defined in the script
    if "window.updateScene = function(data) {" in content:
        print("updateScene is defined.")
    else:
        print("updateScene NOT defined!")

main()
