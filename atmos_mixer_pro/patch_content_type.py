import re

def main():
    path = "lib/features/exhibition/state/three_js_engine_provider.dart"
    with open(path, "r") as f:
        content = f.read()

    content = content.replace(
        '..headers.set("Content-Type", "text/html")',
        '..headers.set("Content-Type", "text/html; charset=utf-8")'
    )

    with open(path, "w") as f:
        f.write(content)

main()
