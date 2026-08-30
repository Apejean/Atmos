import re

def main():
    # 1. Fix three_js_engine_provider.dart
    path = "lib/features/exhibition/state/three_js_engine_provider.dart"
    with open(path, "r") as f:
        content = f.read()

    # Remove the bad setEarLevel outside class
    bad_code = """
  void setEarLevel(double level) {
    if (!isEngineReady) return;
    _webViewController?.runJavaScript(
      "window.updateEarLevel($level);"
    );
  }
"""
    content = content.replace(bad_code, "")

    # Insert it inside the class, right before the last closing brace that belongs to the class
    # The class ends around line 140. Let's find updateSpeakerPosition and insert after it.
    good_code = """  void updateSpeakerPosition(String id, double x, double y, double z, double pan, double tilt) {
    if (!isEngineReady) return;
    _webViewController?.runJavaScript(
      "window.updateSpeaker3DMesh('$id', $x, $y, $z, $pan, $tilt);"
    );
  }

  void setEarLevel(double level) {
    if (!isEngineReady) return;
    _webViewController?.runJavaScript(
      "window.updateEarLevel($level);"
    );
  }"""
    
    content = re.sub(r"  void updateSpeakerPosition\([^\}]+\}\s*\}", good_code, content)
    with open(path, "w") as f:
        f.write(content)


    # 2. Fix speaker_inspector_panel.dart (speaker!.id)
    path = "lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart"
    with open(path, "r") as f:
        content = f.read()
    
    content = content.replace("s.id != speaker.id", "s.id != speaker!.id")
    with open(path, "w") as f:
        f.write(content)


    # 3. Fix preferences_modal.dart
    path = "lib/features/settings/widgets/preferences_modal.dart"
    with open(path, "r") as f:
        content = f.read()
    
    # Replace all "ChannelSetting(" without phaseInvert with "ChannelSetting(phaseInvert: false,"
    content = re.sub(r"ChannelSetting\(\s*([^p])", r"ChannelSetting(phaseInvert: false, \1", content)
    content = re.sub(r"const ChannelSetting\(\s*([^p])", r"const ChannelSetting(phaseInvert: false, \1", content)
    
    with open(path, "w") as f:
        f.write(content)

main()
