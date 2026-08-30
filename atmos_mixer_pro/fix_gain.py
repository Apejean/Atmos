import re

def main():
    path = "lib/features/settings/widgets/preferences_modal.dart"
    with open(path, "r") as f:
        content = f.read()

    # Replace ChannelSetting(phaseInvert: false, ...
    content = re.sub(r"ChannelSetting\(phaseInvert: false,", r"ChannelSetting(phaseInvert: false, gainDb: 0.0,", content)
    
    with open(path, "w") as f:
        f.write(content)

main()
