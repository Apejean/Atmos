import os
import subprocess

def test_rust():
    print("Testing rust compilation")
    result = subprocess.run(["cargo", "check"], cwd="/Users/Allweno/Projects/GitHub/atmos/atmos_mixer_pro/rust", capture_output=True, text=True)
    if result.returncode != 0:
        print(f"Error: {result.stderr}")
    else:
        print("Success")

test_rust()
