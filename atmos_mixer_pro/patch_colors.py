import re

with open("lib/features/exhibition/screens/speaker_canvas_screen.dart", "r") as f:
    content = f.read()

# Replace any occurrence of colors to make sure we perfectly match the dark, sleek DAW style
# The image features a very dark grey/black background with neon blue accents.
# Let's ensure the panel background is dark and sleek.
content = content.replace("color: AppColors.cardSurface,", "color: Color(0xFF1E2128),")
content = content.replace("backgroundColor: AppColors.cardSurface,", "backgroundColor: Color(0xFF1E2128),")
content = content.replace("color: Colors.white24", "color: Color(0xFF32363E)")

with open("lib/features/exhibition/screens/speaker_canvas_screen.dart", "w") as f:
    f.write(content)
