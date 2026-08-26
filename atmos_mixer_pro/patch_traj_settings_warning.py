import re

with open("lib/features/dashboard/widgets/trajectory_settings_modal.dart", "r") as f:
    content = f.read()

content = content.replace("activeColor: AppColors.primaryNeon,", "activeThumbColor: AppColors.primaryNeon,")

with open("lib/features/dashboard/widgets/trajectory_settings_modal.dart", "w") as f:
    f.write(content)
