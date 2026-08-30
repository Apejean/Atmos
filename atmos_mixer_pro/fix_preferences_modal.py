import re

def main():
    path = "lib/features/settings/widgets/preferences_modal.dart"
    with open(path, "r") as f:
        content = f.read()

    # Find ChannelSetting( and add phaseInvert: false if missing
    content = re.sub(r"ChannelSetting\(\s*volume:", "ChannelSetting(phaseInvert: false, volume:", content)
    
    with open(path, "w") as f:
        f.write(content)

main()
