import re

def main():
    path = "lib/features/exhibition/state/three_js_engine_provider.dart"
    with open(path, "r") as f:
        content = f.read()

    # Change to utf8.encode(html)
    content = content.replace("..write(html);", "..add(utf8.encode(html));")
    
    # We need to make sure 'dart:convert' is imported.
    # It probably is, but let's check
    
    with open(path, "w") as f:
        f.write(content)

main()
