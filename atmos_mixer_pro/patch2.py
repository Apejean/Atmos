import os
import glob
import re

pub_cache = os.path.expanduser("~/.pub-cache/hosted/pub.dev/model_viewer_plus*/lib/src/model_viewer_plus_mobile.dart")
files = glob.glob(pub_cache)
file_path = files[-1]
with open(file_path, "r") as f:
    content = f.read()

pattern = r"(await webViewController\.setBackgroundColor\(.*?\);)"
replacement = """try {
      \\1
    } catch (e) {
      debugPrint('Ignored setBackgroundColor error on macOS: $e');
    }"""

content = re.sub(pattern, replacement, content)

with open(file_path, "w") as f:
    f.write(content)
