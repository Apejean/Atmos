import re

def main():
    path = "lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart"
    with open(path, "r") as f:
        content = f.read()

    # Look for the dropdown channel
    match = re.search(r"DropdownButton.*?(channel.*?)\)", content, re.DOTALL)
    if match:
        print("Found channel dropdown")
    else:
        print("Could not find dropdown")

main()
