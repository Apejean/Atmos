import re
import subprocess
import tempfile

def main():
    path = "assets/3d_simulator/studio_engine.html"
    with open(path, "r") as f:
        content = f.read()

    # extract content inside <script> tags
    scripts = re.findall(r'<script>(.*?)</script>', content, re.DOTALL)
    
    js_code = scripts[0] if scripts else ""
    
    with tempfile.NamedTemporaryFile(mode='w', suffix='.js', delete=False) as temp:
        temp.write(js_code)
        temp_name = temp.name

    print("Extracted JS to", temp_name)
    
main()
