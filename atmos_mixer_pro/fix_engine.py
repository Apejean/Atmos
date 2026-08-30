import re

def main():
    path = "lib/features/exhibition/state/three_js_engine_provider.dart"
    with open(path, "r") as f:
        content = f.read()

    new_method = """
  void setEarLevel(double level) {
    executeJavaScript("window.updateEarLevel($level);");
  }

  void dispose() {"""
  
    content = content.replace("  void dispose() {", new_method)

    with open(path, "w") as f:
        f.write(content)

main()
