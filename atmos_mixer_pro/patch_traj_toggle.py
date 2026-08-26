import re

with open("lib/features/exhibition/screens/speaker_canvas_screen.dart", "r") as f:
    content = f.read()

# Remove the Trajectory Layer toggle button from the UI
toggle_pattern = r"""\s*_buildLayerToggle\(\n\s*'Trajectory Layer',\n\s*Icons\.route,\n\s*_showTrajectories,\n\s*\(\) \{\n\s*setState\(\(\) \{\n\s*_showTrajectories = !_showTrajectories;\n\s*if \(!_showTrajectories && _isPlayingAutomation\) \{\n\s*_toggleAutomation\(\); // Pauses if playing\n\s*\}\n\s*\}\);\n\s*\},\n\s*\),"""
content = re.sub(toggle_pattern, "", content)

# Remove the Play Automation button if it's right next to it or related
play_auto_pattern = r"""\s*if \(_showTrajectories\)\n\s*Padding\(\n\s*padding: const EdgeInsets\.only\(left: 16\),\n\s*child: ElevatedButton\.icon\([\s\S]*?child: const Text\('Play Automation'\),\n\s*\),\n\s*\),"""
content = re.sub(play_auto_pattern, "", content)

# Wait, let's just make it simple. We can remove `_showTrajectories` boolean and anything referencing it.
with open("lib/features/exhibition/screens/speaker_canvas_screen.dart", "w") as f:
    f.write(content)
