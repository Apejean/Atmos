import trimesh
import numpy as np

# Load the models
room = trimesh.load('assets/models/room_frame.glb', force='scene')
mannequin = trimesh.load('assets/models/listener_mannequin.glb', force='scene')

print("Mannequin bounds:", mannequin.bounds)
print("Room bounds:", room.bounds)

room_center = (room.bounds[0] + room.bounds[1]) / 2.0
room_center[1] = 0 # keep Y translation at 0 (floor)

mannequin_center = (mannequin.bounds[0] + mannequin.bounds[1]) / 2.0

translation = np.eye(4)
translation[0, 3] = room_center[0] - mannequin_center[0]
translation[2, 3] = room_center[2] - mannequin_center[2]

mannequin.apply_transform(translation)

merged = trimesh.Scene([room, mannequin])
merged.export('assets/models/room_with_listener.glb')
print("Successfully re-centered and merged!")
