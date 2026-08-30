import os

path = '/Users/Allweno/Projects/GitHub/atmos/atmos_mixer_pro/assets/3d_simulator/studio_engine.html'
with open(path, 'r') as f:
    content = f.read()

# Replace the naive room update with a diff check
old_code = '''      if (data.room) {
        currentRoom = data.room;
        buildRoom(currentRoom.width, currentRoom.depth, currentRoom.height);
        buildListenerMannequin(currentRoom.earLevel || 1.2);
      }'''

new_code = '''      if (data.room) {
        const roomChanged = !currentRoom || 
            currentRoom.width !== data.room.width || 
            currentRoom.depth !== data.room.depth || 
            currentRoom.height !== data.room.height ||
            currentRoom.earLevel !== data.room.earLevel;
            
        if (roomChanged) {
            currentRoom = data.room;
            buildRoom(currentRoom.width, currentRoom.depth, currentRoom.height);
            buildListenerMannequin(currentRoom.earLevel || 1.2);
        }
      }'''

content = content.replace(old_code, new_code)

with open(path, 'w') as f:
    f.write(content)
print("Performance patch applied")
