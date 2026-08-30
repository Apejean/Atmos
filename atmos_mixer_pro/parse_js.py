import re
import sys

def main():
    with open("assets/3d_simulator/studio_engine.html", "r") as f:
        html = f.read()
    
    scripts = re.findall(r'<script>(.*?)</script>', html, re.DOTALL)
    for i, script in enumerate(scripts):
        try:
            # We can use node if available, but it wasn't.
            # We can write it to a temp file
            with open(f"temp_script_{i}.js", "w") as sf:
                sf.write(script)
        except Exception as e:
            pass

main()
