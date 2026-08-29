import trimesh

room = trimesh.load('assets/models/room_frame.glb', force='scene')
print("Room bounds:")
print("Min:", room.bounds[0])
print("Max:", room.bounds[1])

mannequin = trimesh.load('assets/models/listener_mannequin.glb', force='scene')
print("Mannequin bounds:")
print("Min:", mannequin.bounds[0])
print("Max:", mannequin.bounds[1])
