import re

def main():
    path = "lib/features/exhibition/state/three_js_engine_provider.dart"
    with open(path, "r") as f:
        content = f.read()

    target = "void updateSpeakerPosition("
    
    idx = content.find(target)
    if idx != -1:
        insert_text = """
  void setSnapEnabled(bool enabled) {
    if (!isEngineReady) return;
    _webViewController?.runJavaScript(
      "window.isSnapEnabled = ${enabled.toString()};"
    );
  }

  """
        content = content[:idx] + insert_text + content[idx:]

    with open(path, "w") as f:
        f.write(content)

main()
