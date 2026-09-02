import os

path = 'lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart'
with open(path, 'r') as f:
    content = f.read()

# Replace speakers.map with speakers.asMap().entries.map to inject index
old_map = """      "speakers": speakers.map((s) => {"""
new_map = """      "speakers": speakers.asMap().entries.map((entry) {
        final s = entry.value;
        return {"""
content = content.replace(old_map, new_map)

# Inject "index"
old_id = """        "id": s.id,"""
new_id = """        "index": entry.key + 1,
        "id": s.id,"""
content = content.replace(old_id, new_id)

# Close the new map block
old_close = """      }).toList(),"""
new_close = """        };
      }).toList(),"""
content = content.replace(old_close, new_close)

with open(path, 'w') as f:
    f.write(content)
print("Dart engine patched.")
