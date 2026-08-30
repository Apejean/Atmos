import re

def main():
    path = "lib/features/exhibition/state/three_js_engine_provider.dart"
    with open(path, "r") as f:
        content = f.read()

    old_bg = "webController.setBackgroundColor(const Color(0xFF0B0F14));"
    new_bg = """try {
      webController.setBackgroundColor(const Color(0xFF0B0F14));
    } catch (e) {
      debugPrint("macOS setBackgroundColor error ignored: $e");
    }"""

    content = content.replace(old_bg, new_bg)

    with open(path, "w") as f:
        f.write(content)

main()
