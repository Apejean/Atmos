import os

path = "lib/features/exhibition/screens/speaker_canvas_screen.dart"
if os.path.exists(path):
    with open(path, "r") as f:
        content = f.read()
    content = content.replace("AppColors.surface", "Color(0xFF23252A)")
    with open(path, "w") as f:
        f.write(content)
