import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# 1. We should replace the whole Positioned.fill( child: ModelViewer(...) ) with one that uses _localGlbPath
old_mv = """          // 1. Core 3D Orbit View
          Positioned.fill(
            child: ModelViewer("""

# We need to find the end of ModelViewer.
# Let's just use regex to replace everything between ModelViewer( and ), inclusive?
# It's easier to write a targeted replace.

# Remove the scale property
content = re.sub(r"scale:\s*'\$\{roomWidth[^}]+\}\s*\$\{roomHeight[^}]+\}\s*\$\{roomDepth[^}]+\}',", "", content)

# Remove the relatedJs hack!
related_js_regex = r"relatedJs:\s*'''(?:[^']|'(?!''))*''',"
content = re.sub(related_js_regex, "", content)

# Change src to use _localGlbPath
content = re.sub(r"src:\s*'assets/models/room_with_listener.glb',", "src: _localGlbPath != null ? 'file://${_localGlbPath}' : 'assets/models/room_with_listener.glb',", content)

# Handle the case where _localGlbPath is not ready
old_positioned = """          // 1. Core 3D Orbit View
          Positioned.fill(
            child: ModelViewer("""
new_positioned = """          // 1. Core 3D Orbit View
          Positioned.fill(
            child: _localGlbPath == null ? const Center(child: CircularProgressIndicator()) : ModelViewer("""
content = content.replace(old_positioned, new_positioned)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
