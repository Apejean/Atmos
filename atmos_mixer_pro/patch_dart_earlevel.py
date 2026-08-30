import re

def main():
    path = "lib/features/exhibition/state/three_js_engine_provider.dart"
    with open(path, "r") as f:
        content = f.read()

    # Add setEarLevel
    if "void setEarLevel(" not in content:
        content = content.replace(
            "void setSnapEnabled(bool enabled)",
            "void setEarLevel(double level) {\n    if (!isEngineReady) return;\n    _webViewController?.runJavaScript(\"window.updateEarLevel($level);\");\n  }\n\n  void setSnapEnabled(bool enabled)"
        )

    with open(path, "w") as f:
        f.write(content)

main()
