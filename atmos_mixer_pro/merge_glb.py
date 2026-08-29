import trimesh

# Load both scenes
room = trimesh.load('assets/models/room_frame.glb', force='scene')
mannequin = trimesh.load('assets/models/listener_mannequin.glb', force='scene')

# The mannequin should be scaled appropriately? Or is it already the right size?
# Let's just append the mannequin geometry into the room scene
merged = trimesh.Scene([room, mannequin])

# Export
merged.export('assets/models/room_with_listener.glb')
print("Successfully merged room_frame and listener_mannequin into room_with_listener.glb")
