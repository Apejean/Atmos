import re
import os

def fix_channel_setting(path):
    if os.path.exists(path):
        with open(path, "r") as f:
            content = f.read()
        content = re.sub(r"ChannelSetting\(\s*enabled:\s*([^,]+),\s*customName:\s*([^,]+),\s*delayMs:\s*([^,]+),\s*eqBands:\s*([^,]+),?\s*\)", r"ChannelSetting(enabled: \1, customName: \2, delayMs: \3, eqBands: \4, gainDb: 0.0, phaseInvert: false)", content)
        content = re.sub(r"ChannelSetting\(\s*enabled:\s*([^,]+),\s*customName:\s*([^,]+),\s*delayMs:\s*([^,]+),?\s*\)", r"ChannelSetting(enabled: \1, customName: \2, delayMs: \3, gainDb: 0.0, phaseInvert: false)", content)
        content = re.sub(r"ChannelSetting\(\s*enabled:\s*([^,]+),\s*customName:\s*([^,]+),?\s*\)", r"ChannelSetting(enabled: \1, customName: \2, gainDb: 0.0, phaseInvert: false)", content)
        content = re.sub(r"ChannelSetting\(\s*enabled:\s*([^,]+),?\s*\)", r"ChannelSetting(enabled: \1, gainDb: 0.0, phaseInvert: false)", content)
        with open(path, "w") as f:
            f.write(content)

fix_channel_setting("lib/features/settings/widgets/preferences_modal.dart")
fix_channel_setting("test/dashboard_features_test.dart")
fix_channel_setting("test/master_verification_items_7_to_20_test.dart")
