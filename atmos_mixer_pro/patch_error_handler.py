import re

def main():
    path = "assets/3d_simulator/studio_engine.html"
    with open(path, "r") as f:
        content = f.read()

    new_script = """  <script>
    window.onerror = function(msg, url, lineNo, columnNo, error) {
      console.error("Global Error: " + msg + " at line " + lineNo);
      return false;
    };"""

    content = content.replace("  <script>", new_script)

    with open(path, "w") as f:
        f.write(content)

main()
