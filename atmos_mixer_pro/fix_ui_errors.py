import re
import os

# 1. Fix surfaceLighter in speaker_canvas_screen.dart
path = "lib/features/exhibition/screens/speaker_canvas_screen.dart"
if os.path.exists(path):
    with open(path, "r") as f:
        content = f.read()
    content = content.replace("AppColors.surfaceLighter", "AppColors.surface")
    with open(path, "w") as f:
        f.write(content)

# 2. Fix preferences_modal.dart missing gainDb, phaseInvert
path2 = "lib/features/settings/widgets/preferences_modal.dart"
if os.path.exists(path2):
    with open(path2, "r") as f:
        content2 = f.read()
    content2 = re.sub(r"OutputChannelModel\(\s*id:\s*([^,]+),\s*name:\s*([^,]+)\s*\)", r"OutputChannelModel(id: \1, name: \2, gainDb: 0.0, phaseInvert: false)", content2)
    # the errors might be OutputChannelModel or ChannelSetting depending on what it is
    # Let's just do a brute force fix if it's OutputChannelModel
    content2 = re.sub(r"OutputChannelModel\(\s*id:\s*([^,]+),\s*name:\s*([^,]+),\s*delayMs:\s*([^,]+)\s*\)", r"OutputChannelModel(id: \1, name: \2, delayMs: \3, gainDb: 0.0, phaseInvert: false)", content2)
    with open(path2, "w") as f:
        f.write(content2)

# 3. Fix tests
paths = ["test/dashboard_features_test.dart", "test/master_verification_items_7_to_20_test.dart"]
for p in paths:
    if os.path.exists(p):
        with open(p, "r") as f:
            c = f.read()
        c = re.sub(r"OutputChannelModel\(\s*id:\s*([^,]+),\s*name:\s*([^,]+)\s*\)", r"OutputChannelModel(id: \1, name: \2, gainDb: 0.0, phaseInvert: false)", c)
        c = re.sub(r"OutputChannelModel\(\s*id:\s*([^,]+),\s*name:\s*([^,]+),\s*delayMs:\s*([^,]+)\s*\)", r"OutputChannelModel(id: \1, name: \2, delayMs: \3, gainDb: 0.0, phaseInvert: false)", c)
        with open(p, "w") as f:
            f.write(c)

