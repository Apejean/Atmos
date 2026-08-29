import os
import glob
import re

pub_cache = os.path.expanduser("~/.pub-cache/hosted/pub.dev/model_viewer_plus*/lib/src/model_viewer_plus_mobile.dart")
files = glob.glob(pub_cache)
file_path = files[-1]
with open(file_path, "r") as f:
    content = f.read()

# Fix "setState() called after dispose()" inside the then() block
pattern = r"(\.then\(\(String html\) \{\s*)(setState\(\(\) \{)(.*?)(proxy = _Proxy\(\);\s*\})(.*?)(\}\);\s*\})"
replacement = """\\1if (mounted) { \\2\\3\\4\\5\\6 }"""

# Or a more robust replace for setState() related to proxy
content = re.sub(r"(setState\(\(\) \{\s*proxy = _Proxy\(\);\s*\}\);)", r"if (mounted) {\n      \1\n    }", content)

with open(file_path, "w") as f:
    f.write(content)

