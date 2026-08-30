import re

def main():
    path = "lib/features/exhibition/state/three_js_engine_provider.dart"
    with open(path, "r") as f:
        content = f.read()

    new_method = """
  void setEarLevel(double level) {
    if (!isEngineReady) return;
    _webViewController?.runJavaScript(
      "window.updateEarLevel($level);"
    );
  }
"""
    # Insert right before the last closing brace of the class
    idx = content.rfind("}")
    content = content[:idx] + new_method + content[idx:]

    with open(path, "w") as f:
        f.write(content)

main()
