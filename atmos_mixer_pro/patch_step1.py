import re

path = "lib/features/exhibition/screens/speaker_canvas_screen.dart"
with open(path, "r") as f:
    content = f.read()

# 1. Remove all emojis
content = content.replace("🌡️ SPL Heatmap", "SPL HEATMAP")
content = content.replace("📄 Export PDF Report", "EXPORT PDF REPORT")
content = content.replace("📄 Export Rigging Report", "EXPORT RIGGING REPORT")
content = content.replace("🎯 Apply 3D Calibration", "APPLY 3D CALIBRATION")
content = content.replace("📏 Room Setup", "ROOM SETUP")
content = content.replace("🔊 Speaker Inspector", "SPEAKER INSPECTOR")
content = content.replace("🎧 Virtual (Binaural)", "Virtual (Binaural)")
content = content.replace("🔊 Physical (Direct Out)", "Physical (Direct Out)")
content = content.replace("🌡️", "")
content = content.replace("📄", "")
content = content.replace("🎯", "")
content = content.replace("📏", "")
content = content.replace("🔊", "")
content = content.replace("🎧", "")

with open(path, "w") as f:
    f.write(content)
