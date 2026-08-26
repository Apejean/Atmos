import re

with open("lib/features/dashboard/screens/dashboard_screen.dart", "r") as f:
    content = f.read()

# Remove the 1-click calibration button since it's breaking build anyway (we deleted the widget earlier but Front agent might have re-added or I missed it)
calib_pattern = r"""\s*IconButton\(\n\s*icon: const Icon\(Icons\.tune, color: Colors\.cyanAccent\),\n\s*tooltip: '1-Click Auto Calibration',\n\s*onPressed: \(\) \{\n\s*showDialog\(\n\s*context: context,\n\s*builder: \(context\) => const AutoCalibrationModal\(\),\n\s*\);\n\s*\},\n\s*\),"""
content = re.sub(calib_pattern, "", content)

with open("lib/features/dashboard/screens/dashboard_screen.dart", "w") as f:
    f.write(content)
