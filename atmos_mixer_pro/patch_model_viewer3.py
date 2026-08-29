import os
import glob
import re

pub_cache = os.path.expanduser("~/.pub-cache/hosted/pub.dev/model_viewer_plus*/lib/src/model_viewer_plus_mobile.dart")
files = glob.glob(pub_cache)
file_path = files[-1]
with open(file_path, "r") as f:
    content = f.read()

# Fix "setState() called after dispose()"
pattern = r"(setState\(\(\) \{\s*proxy = _Proxy\(\);\s*\}\);)"
replacement = """if (mounted) {
      \\1
    }"""
content = re.sub(pattern, replacement, content)

with open(file_path, "w") as f:
    f.write(content)

