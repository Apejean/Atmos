import re

with open('lib/features/exhibition/models/room_zone.dart', 'r') as f:
    content = f.read()

# Fix toJson keys for physicalWidth and physicalHeight (they were 'physicalWidth' instead of 'physical_width' in some old JSON parsing logic perhaps)
# Let's check toJson

old_json = """      'physicalWidth': physicalWidth,
      'physicalHeight': physicalHeight,
      'ceiling_height': ceilingHeight,"""

new_json = """      'physical_width': physicalWidth,
      'physical_height': physicalHeight,
      'ceiling_height': ceilingHeight,"""
      
content = content.replace(old_json, new_json)

with open('lib/features/exhibition/models/room_zone.dart', 'w') as f:
    f.write(content)
