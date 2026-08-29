import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# 1. Fix maxCameraOrbit logic
max_orbit_old = "maxCameraOrbit: 'auto auto 25m',"
content = content.replace(max_orbit_old, "maxCameraOrbit: 'auto auto 100m',")

# 2. Fix mannequin initial stretching
# Ensure JS applies scaling BEFORE first frame render by hooking into 'model-visibility' or just 'load' better
# Change event listener from 'load' to 'model-visibility' or 'scene-graph-ready' to prevent FOUC (flash of unstyled content)
old_js = """
                document.querySelector('model-viewer').addEventListener('load', function(e) {
                  const mv = e.target;
"""
new_js = """
                const mv = document.querySelector('model-viewer');
                mv.addEventListener('scene-graph-ready', function(e) {
"""
content = content.replace(old_js, new_js)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
