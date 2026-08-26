import re

# Fix viewport_3d Type
with open('lib/features/exhibition/widgets/viewport_3d/room_3d_viewport.dart', 'r') as f:
    content = f.read()

content = content.replace('BlueprintState blueprint', 'BlueprintData blueprint')

# Fix room list access (BlueprintData does not have rooms! The old BlueprintState did in another version, but here RoomZone might be managed elsewhere, or BlueprintData doesn't have it.)
